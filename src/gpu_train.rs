//! GPU training of an NNUE network with `burn`'s Wgpu backend (Vulkan/Metal/DX12).
//!
//! This is a real autodiff training loop on the graphics adapter detected by
//! [`crate::gpu`] — unlike CUDA/ROCm-only paths it works on AMD/Intel cards
//! through Vulkan. The data is TKD1 self-play; the exported file is a standard
//! TKN1 v2 network that the engine can load.

use crate::eval::nnue::Network;
use burn::module::{Module, Param, ParamId};
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use burn::prelude::*;
use burn::tensor::Distribution;
use burn::tensor::{Int, Tensor, TensorData};
use serde_json::Value;
use std::path::Path;

/// Backend used for training: autodiff over `burn::backend::Wgpu`.
pub type TrainBackend = burn::backend::Autodiff<burn::backend::Wgpu>;

const HEADER: &[u8] = b"TKD1\x01\x00\x00\x00";
const REC_SIZE: usize = 1 + 1 + 4 + 4 + 2 + 4 + 2;

/// One self-play sample with its piece list.
struct Sample {
    stm: u8,
    result: i8,
    score: i32,
    pieces: Vec<(u16, u16, u8)>, // square, piece type, color
}

fn read_u16(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn read_u32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

fn read_samples(dir: &Path) -> Result<Vec<Sample>, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("samples_"))
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no samples_*.bin in {}", dir.display()));
    }
    let mut out = Vec::new();
    for path in files {
        let buf = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if buf.len() < 8 || &buf[..8] != HEADER {
            return Err(format!("{}: not TKD1 v1", path.display()));
        }
        let mut p = 8;
        while p + REC_SIZE <= buf.len() {
            let stm = buf[p];
            let result = buf[p + 1] as i8;
            let score = read_u32(&buf, p + 2) as i32;
            let n = read_u16(&buf, p + 16) as usize;
            p += REC_SIZE;
            if p + 4 * n > buf.len() {
                return Err(format!("{}: truncated record", path.display()));
            }
            let mut pieces = Vec::with_capacity(n);
            for i in 0..n {
                let packed = read_u16(&buf, p + 4 * i + 2);
                pieces.push((
                    read_u16(&buf, p + 4 * i),
                    packed & 0x1FF,
                    (packed >> 9) as u8,
                ));
            }
            p += 4 * n;
            out.push(Sample {
                stm,
                result,
                score,
                pieces,
            });
        }
    }
    Ok(out)
}

/// Architecture constants from the single source of truth.
struct ArchInfo {
    classes: usize,
    class_map: Vec<u8>,
    thresholds: Vec<u16>,
    eval_limit: i32,
}

fn read_arch() -> Result<ArchInfo, String> {
    let v: Value = serde_json::from_str(include_str!("../rules/nnue_arch.json"))
        .map_err(|e| format!("nnue_arch.json: {e}"))?;
    let classes = v["num_classes"].as_u64().unwrap_or(27) as usize;
    let class_map = v["class_map"]
        .as_array()
        .ok_or("class_map missing")?
        .iter()
        .map(|x| x.as_u64().unwrap_or(0) as u8)
        .collect();
    let thresholds = v["thresholds_pieces"]
        .as_array()
        .ok_or("thresholds_pieces missing")?
        .iter()
        .map(|x| x.as_u64().unwrap_or(0) as u16)
        .collect();
    Ok(ArchInfo {
        classes,
        class_map,
        thresholds,
        eval_limit: v["eval_limit"].as_i64().unwrap_or(250_000) as i32,
    })
}

fn bucket_of(npieces: usize, thresholds: &[u16]) -> usize {
    thresholds
        .iter()
        .filter(|&&t| (npieces as u16) >= t)
        .count()
}

/// Dense feature vectors for both perspectives, plus metadata.
struct Batch {
    x0: Vec<f32>,
    x1: Vec<f32>,
    stm: Vec<i64>,
    bucket: Vec<i64>,
    target: Vec<f32>,
}

fn make_batches(
    samples: &[Sample],
    arch: &ArchInfo,
    batch_size: usize,
    lam: f32,
    sc: f32,
) -> Vec<Batch> {
    let dims = arch.classes * 2 * 1296;
    let mut out = Vec::new();
    for chunk in samples.chunks(batch_size) {
        let b = chunk.len();
        let (mut x0, mut x1) = (vec![0.0; dims * b], vec![0.0; dims * b]);
        let (mut stm, mut bucket, mut target) = (
            Vec::with_capacity(b),
            Vec::with_capacity(b),
            Vec::with_capacity(b),
        );
        for (i, s) in chunk.iter().enumerate() {
            for &(sq, pt, color) in &s.pieces {
                let class = arch.class_map.get(pt as usize).copied().unwrap_or(0) as usize;
                for (p, dst) in [(0usize, &mut x0), (1usize, &mut x1)] {
                    let rot = if p == 1 {
                        1295usize.saturating_sub(sq as usize)
                    } else {
                        sq as usize
                    };
                    let plane = if color as usize == p {
                        class
                    } else {
                        arch.classes + class
                    };
                    let idx = plane * 1296 + rot;
                    if idx < dims {
                        dst[i * dims + idx] = 1.0;
                    }
                }
            }
            stm.push(s.stm as i64);
            bucket.push(bucket_of(s.pieces.len(), &arch.thresholds) as i64);
            let sig = 1.0 / (1.0 + (-s.score as f32 / sc).exp());
            let res = (s.result as f32 + 1.0) / 2.0;
            target.push(lam * sig + (1.0 - lam) * res);
        }
        out.push(Batch {
            x0,
            x1,
            stm,
            bucket,
            target,
        });
    }
    out
}

/// NNUE model in burn, architecture-compatible with TKN1 v2.
#[derive(Module, Debug)]
struct NnueModel<B: Backend> {
    ft: Param<Tensor<B, 2>>,
    ft_bias: Param<Tensor<B, 1>>,
    psqt: Param<Tensor<B, 2>>,
    fc0_w: Param<Tensor<B, 3>>,
    fc0_b: Param<Tensor<B, 2>>,
    fc1_w: Param<Tensor<B, 3>>,
    fc1_b: Param<Tensor<B, 2>>,
    fc2_w: Param<Tensor<B, 2>>,
    fc2_b: Param<Tensor<B, 1>>,
}

impl<B: Backend> NnueModel<B> {
    fn init(device: &B::Device, dims: usize, _classes: usize) -> Self {
        let normal = Distribution::Normal(0.0, 0.05);
        let t = |shape: [usize; 2], d| {
            Param::initialized(ParamId::new(), Tensor::<B, 2>::random(shape, d, device))
        };
        let t3 = |shape: [usize; 3], d| {
            Param::initialized(ParamId::new(), Tensor::<B, 3>::random(shape, d, device))
        };
        Self {
            ft: t([dims, 256], normal),
            ft_bias: Param::initialized(ParamId::new(), Tensor::<B, 1>::full([256], 8.0, device)),
            psqt: Param::initialized(ParamId::new(), Tensor::<B, 2>::zeros([dims, 8], device)),
            fc0_w: t3([8, 256, 16], normal),
            fc0_b: Param::initialized(ParamId::new(), Tensor::<B, 2>::zeros([8, 16], device)),
            fc1_w: t3([8, 16, 32], normal),
            fc1_b: Param::initialized(ParamId::new(), Tensor::<B, 2>::zeros([8, 32], device)),
            fc2_w: t([8, 32], normal),
            fc2_b: Param::initialized(ParamId::new(), Tensor::<B, 1>::zeros([8], device)),
        }
    }

    fn forward(
        &self,
        x0: Tensor<B, 2>,
        x1: Tensor<B, 2>,
        stm: Tensor<B, 1, Int>,
        bucket: Tensor<B, 1, Int>,
    ) -> Tensor<B, 1> {
        let bias = self.ft_bias.val().unsqueeze();
        let a0 = (x0.clone().matmul(self.ft.val()) + bias.clone()).clamp(0.0, 127.0);
        let a1 = (x1.clone().matmul(self.ft.val()) + bias).clamp(0.0, 127.0);
        let b0dim = x0.dims()[0];
        let black = stm
            .clone()
            .equal_elem(0)
            .reshape([b0dim, 1])
            .expand::<2, _>([b0dim, 256]);
        let us = a0.clone().mask_where(black.clone(), a1.clone());
        let them = a1.mask_where(black, a0);
        let pw = us.mul(them).div_scalar(128.0);
        let b = b0dim;

        let w0 = self.fc0_w.val().select(0, bucket.clone());
        let b0 = self.fc0_b.val().select(0, bucket.clone());
        let h0 = pw
            .reshape([b, 1, 256])
            .matmul(w0)
            .squeeze::<2>()
            .add(b0)
            .clamp(0.0, 127.0);
        let w1 = self.fc1_w.val().select(0, bucket.clone());
        let b1 = self.fc1_b.val().select(0, bucket.clone());
        let h1 = h0
            .reshape([b, 1, 16])
            .matmul(w1)
            .squeeze::<2>()
            .add(b1)
            .clamp(0.0, 127.0);
        let w2 = self.fc2_w.val().select(0, bucket.clone());
        let b2 = self.fc2_b.val().select(0, bucket.clone());
        let dense = h1.mul(w2).sum_dims_squeeze::<1, _>(&[1]).add(b2);

        let p0 = x0
            .clone()
            .matmul(self.psqt.val())
            .gather(1, bucket.clone().reshape([b, 1]))
            .squeeze::<1>();
        let p1 = x1
            .clone()
            .matmul(self.psqt.val())
            .gather(1, bucket.reshape([b, 1]))
            .squeeze::<1>();
        let stm_black = stm.equal_elem(0);
        let psqt = p0
            .clone()
            .sub(p1.clone())
            .mask_where(stm_black, p1.sub(p0))
            .div_scalar(2.0);
        psqt.add(dense)
    }
}

/// Train a network on TKD1 data using the GPU, then save raw TKN1.
pub fn run(
    data: &Path,
    out: &Path,
    epochs: usize,
    batch_size: usize,
    max_samples: usize,
    lr: f32,
) -> Result<(), String> {
    let arch = read_arch()?;
    let samples = read_samples(data)?;
    if samples.len() < 32 {
        return Err(format!("only {} samples; need >= 32", samples.len()));
    }
    let dims = arch.classes * 2 * 1296;
    let sc = 20_000.0f32;
    // Bound the subset so a quick GPU run does not allocate tens of GB of
    // dense features (~286 MB per 512-sample batch at 69,984 dims).
    let max_samples = samples.len().min(max_samples.max(32));
    let samples = &samples[..max_samples];
    println!(
        "training on {} samples ({} batches/epoch, batch={})",
        samples.len(),
        samples.len().div_ceil(batch_size),
        batch_size
    );
    let batches = make_batches(samples, &arch, batch_size, 0.7, sc);
    let device = <TrainBackend as burn::tensor::backend::BackendTypes>::Device::default();
    let mut model: NnueModel<TrainBackend> = NnueModel::init(&device, dims, arch.classes);
    let mut optim = AdamConfig::new().init::<TrainBackend, NnueModel<TrainBackend>>();

    for epoch in 0..epochs {
        let mut loss_sum = 0.0;
        for batch in &batches {
            let b = batch.x0.len() / dims;
            let x0 = Tensor::<TrainBackend, 2>::from_data(
                TensorData::new(batch.x0.clone(), [b, dims]),
                &device,
            );
            let x1 = Tensor::<TrainBackend, 2>::from_data(
                TensorData::new(batch.x1.clone(), [b, dims]),
                &device,
            );
            let stm = Tensor::<TrainBackend, 1, Int>::from_data(
                TensorData::new(batch.stm.clone(), [b]),
                &device,
            );
            let bucket = Tensor::<TrainBackend, 1, Int>::from_data(
                TensorData::new(batch.bucket.clone(), [b]),
                &device,
            );
            let target = Tensor::<TrainBackend, 1>::from_data(
                TensorData::new(batch.target.clone(), [b]),
                &device,
            );
            let pred = model.forward(x0, x1, stm, bucket);
            let loss = pred.sub(target).powf_scalar(2.0).sum();
            let ld = loss.clone().into_data().convert::<f64>();
            loss_sum += ld.as_slice().unwrap().first().copied().unwrap_or(0.0) as f64;
            let grads = loss.backward();
            let params = GradientsParams::from_grads(grads, &model);
            model = optim.step(lr as f64, model, params);
        }
        println!(
            "epoch {}/{}: mse={:.5}",
            epoch + 1,
            epochs,
            loss_sum / batches.len() as f64
        );
    }

    // Quantise the trained float weights and write a standard TKN1 v2 file.
    let net = quantize(&model, &arch)?;
    net.save(out)
        .map_err(|e| format!("cannot save {}: {e}", out.display()))?;
    println!("wrote {} (TKN1 v2, trained on GPU)", out.display());
    Ok(())
}

fn to_vec<B: Backend, const D: usize>(t: Tensor<B, D>) -> Vec<f32> {
    t.into_data().convert::<f32>().as_slice().unwrap().to_vec()
}

fn qi16(v: f32, scale: f32) -> i16 {
    (v * scale).round().clamp(-32767.0, 32767.0) as i16
}
fn qi32(v: f32, scale: f32) -> i32 {
    (v * scale)
        .round()
        .clamp(-(i32::MAX as f32), i32::MAX as f32) as i32
}
fn qi8(v: f32, scale: f32) -> i8 {
    (v * scale).round().clamp(-127.0, 127.0) as i8
}

/// Quantise the trained float weights into a standard TKN1 v2 `Network`.
fn quantize(model: &NnueModel<TrainBackend>, arch: &ArchInfo) -> Result<Network, String> {
    let ft = to_vec(model.ft.val())
        .iter()
        .map(|&v| qi16(v, 64.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let ft_bias = to_vec(model.ft_bias.val())
        .iter()
        .map(|&v| qi32(v, 64.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let psqt = to_vec(model.psqt.val())
        .iter()
        .map(|&v| qi32(v, 16.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let fc0_w = to_vec(model.fc0_w.val())
        .iter()
        .map(|&v| qi8(v, 64.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let fc0_b = to_vec(model.fc0_b.val())
        .iter()
        .map(|&v| qi32(v, 64.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let fc1_w = to_vec(model.fc1_w.val())
        .iter()
        .map(|&v| qi8(v, 64.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let fc1_b = to_vec(model.fc1_b.val())
        .iter()
        .map(|&v| qi32(v, 64.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let fc2_w = to_vec(model.fc2_w.val())
        .iter()
        .map(|&v| qi8(v, 16.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let fc2_b = to_vec(model.fc2_b.val())
        .iter()
        .map(|&v| qi32(v, 16.0))
        .collect::<Vec<_>>()
        .into_boxed_slice();

    let spec = crate::eval::nnue::arch::ArchSpec::embedded();
    let mut thresholds = [0u16; crate::eval::nnue::arch::NUM_THRESHOLDS];
    thresholds.copy_from_slice(&spec.thresholds_pieces);
    Ok(Network {
        planes: spec.planes(),
        class_map: spec.class_map.clone(),
        thresholds,
        arch_hash: spec.arch_hash(),
        eval_limit: arch.eval_limit,
        tt_salt: rand::random(),
        ft,
        ft_bias,
        psqt,
        fc0_w,
        fc0_b,
        fc1_w,
        fc1_b,
        fc2_w,
        fc2_b,
    })
}
