//! Single source of truth of the NNUE architecture.
//!
//! `rules/nnue_arch.json` is embedded at compile time and read at runtime by
//! the Python training tools, so both sides always agree on the class map,
//! the output-bucket thresholds and the quantisation constants.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

use super::{FT_SHIFT, FV_SCALE, L1, NSQ, OUT_BUCKETS};

/// The embedded architecture file.
pub const ARCH_JSON: &str = include_str!("../../../rules/nnue_arch.json");

/// Number of output-bucket thresholds (`OUT_BUCKETS - 1`).
pub const NUM_THRESHOLDS: usize = OUT_BUCKETS - 1;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ArchSpec {
    pub format: String,
    pub version: u16,
    pub num_squares: usize,
    pub l1: usize,
    pub out_buckets: usize,
    pub ft_shift: u32,
    pub fv_scale: i32,
    /// Evaluations are clamped to `[-eval_limit, eval_limit]`.
    pub eval_limit: i32,
    pub num_classes: usize,
    /// `class_map[piece_type]`, entry 0 unused.
    pub class_map: Vec<u8>,
    /// Ascending piece-count thresholds that select the output bucket.
    pub thresholds_pieces: Vec<u16>,
    /// Mean material value per class; a training hint (initialises the PSQT
    /// as a material count), not part of the architecture hash.
    #[serde(default)]
    pub class_values: Vec<i32>,
}

impl ArchSpec {
    /// The architecture compiled into this binary.
    pub fn embedded() -> &'static ArchSpec {
        static SPEC: OnceLock<ArchSpec> = OnceLock::new();
        SPEC.get_or_init(|| {
            ArchSpec::from_json(ARCH_JSON)
                .unwrap_or_else(|e| panic!("rules/nnue_arch.json is invalid: {e}"))
        })
    }

    pub fn from_json(json: &str) -> Result<ArchSpec, String> {
        let spec: ArchSpec = serde_json::from_str(json).map_err(|e| e.to_string())?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), String> {
        let check = |ok: bool, msg: &str| if ok { Ok(()) } else { Err(msg.to_string()) };
        check(self.format == "TKN1", "format must be TKN1")?;
        check(self.version == 2, "version must be 2")?;
        check(
            self.num_squares == NSQ,
            "num_squares differs from the engine",
        )?;
        check(self.l1 == L1, "l1 differs from the engine")?;
        check(
            self.out_buckets == OUT_BUCKETS,
            "out_buckets differs from the engine",
        )?;
        check(
            self.ft_shift == FT_SHIFT,
            "ft_shift differs from the engine",
        )?;
        check(
            self.fv_scale == FV_SCALE,
            "fv_scale differs from the engine",
        )?;
        check(self.eval_limit > 0, "eval_limit must be positive")?;
        check(
            self.class_map.len() == crate::pieces::num_piece_types() + 1,
            "class_map length must be num_piece_types + 1",
        )?;
        check(self.class_map[0] == 0, "class_map[0] must be 0")?;
        check(
            super::classes::num_classes(&self.class_map) == self.num_classes,
            "num_classes does not match class_map",
        )?;
        check(
            self.class_values.is_empty() || self.class_values.len() == self.num_classes,
            "class_values must have one entry per class",
        )?;
        check(
            self.thresholds_pieces.len() == NUM_THRESHOLDS,
            "need OUT_BUCKETS-1 thresholds",
        )?;
        check(
            self.thresholds_pieces.windows(2).all(|w| w[0] < w[1]),
            "thresholds must be strictly ascending",
        )?;
        Ok(())
    }

    /// Number of feature planes: own and enemy plane for every class.
    pub fn planes(&self) -> usize {
        2 * self.num_classes
    }

    /// Number of input features per perspective.
    pub fn dims(&self) -> usize {
        self.planes() * NSQ
    }

    /// Canonical architecture descriptor. Python builds the identical string.
    pub fn descriptor(&self) -> String {
        let cmap = Sha256::digest(&self.class_map);
        let cmap_hex: String = cmap[..8].iter().map(|b| format!("{b:02x}")).collect();
        let th: Vec<String> = self
            .thresholds_pieces
            .iter()
            .map(|t| t.to_string())
            .collect();
        format!(
            "taikyoku-nnue-S;kb=1;planes={};sq={};L1={};ob={};acc=i32;ftshift={};\
act=clip0_127;pairwise=shr7;stack=16-32-1;classes={};cmap={};buckets=[{}]",
            self.planes(),
            self.num_squares,
            self.l1,
            self.out_buckets,
            self.ft_shift,
            self.num_classes,
            cmap_hex,
            th.join(", ")
        )
    }

    /// SHA-256 of the descriptor; stored in every network file.
    pub fn arch_hash(&self) -> [u8; 32] {
        Sha256::digest(self.descriptor().as_bytes()).into()
    }
}
