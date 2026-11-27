//! GPU adapter detection via WebGPU (`wgpu`).
//!
//! CUDA and ROCm only see GPUs that expose their respective compute runtimes.
//! `wgpu` enumerates *graphics* adapters too — Vulkan (AMD, Intel, NVIDIA),
//! Metal (Apple Silicon / Intel Macs), DirectX 12 and OpenGL — so this module
//! can answer "is there any GPU here at all?" even on a box without CUDA or
//! ROCm installed. It is detection/reporting only: it never initialize a
//! training backend on its own, and it works with no feature other than
//! `gpu-detect`.

use serde::Serialize;

/// One GPU/graphics adapter reported by `wgpu`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Adapter {
    /// `vulkan`, `metal`, `dx12`, `gl`, `browser-webgpu`, …
    pub backend: String,
    /// Human-readable device name (`description` or `name`).
    pub name: String,
    /// PCI vendor / device ids when reported (`None` for e.g. Metal).
    pub vendor_device: Option<(u32, u32)>,
    /// Driver/version string, if reported.
    pub driver: Option<String>,
    /// Whether this adapter exposes general-purpose compute (WebGPU spec).
    pub compute: bool,
}

impl Adapter {
    /// Short one-line description for logs / the diagnostics panel.
    #[must_use]
    pub fn summary(&self) -> String {
        let ids = self
            .vendor_device
            .map_or_else(String::new, |(v, d)| format!(" [{v:04x}:{d:04x}]"));
        format!(
            "{}{ids} via {}{}",
            self.name,
            self.backend,
            if self.compute { " (compute)" } else { "" }
        )
    }
}

/// Enumerate all available adapters, newest API first.
///
/// Returns an empty list (not an error) when no graphics adapter exists — a
/// headless server is a valid configuration. Errors are only for a broken
/// `wgpu` instance, which should never happen with the all-backends descriptor.
#[must_use]
pub fn detect() -> Result<Vec<Adapter>, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        flags: wgpu::InstanceFlags::default(),
        dx12_shader_compiler: wgpu::Dx12Compiler::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    });
    let adapters = instance.enumerate_adapters(wgpu::Backends::all());
    Ok(adapters
        .into_iter()
        .map(|a| {
            let info = a.get_info();
            let name = info.name.clone();
            let driver = if info.driver_info.is_empty() {
                if info.driver.is_empty() {
                    None
                } else {
                    Some(info.driver.clone())
                }
            } else {
                Some(info.driver_info.clone())
            };
            Adapter {
                backend: format!("{:?}", info.backend),
                name,
                vendor_device: if info.vendor == 0 && info.device == 0 {
                    None
                } else {
                    Some((info.vendor, info.device))
                },
                driver,
                // WebGPU requires compute-shader support on every adapter;
                // wgpu has no separate "compute" feature flag to check.
                compute: true,
            }
        })
        .collect())
}

/// Like [`detect`], but never fails: on error returns an empty list.
#[must_use]
pub fn detect_or_empty() -> Vec<Adapter> {
    detect().unwrap_or_default()
}

/// Human-readable summary of all adapters, one per line.
#[must_use]
pub fn report() -> String {
    let adapters = detect_or_empty();
    if adapters.is_empty() {
        return "no GPU/graphics adapter detected (wgpu: Vulkan/Metal/DX12/GL)".to_owned();
    }
    adapters
        .iter()
        .enumerate()
        .map(|(i, a)| format!("  [{i}] {}", a.summary()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_never_fails_and_summaries_are_non_empty() {
        // On a headless CI box detect() legitimately returns an empty list.
        for a in detect_or_empty() {
            assert!(!a.summary().is_empty());
            assert!(!a.backend.is_empty());
        }
        assert!(!report().is_empty());
    }

    #[test]
    fn adapter_summary_includes_backend() {
        let a = Adapter {
            backend: "vulkan".into(),
            name: "Radeon RX 6800".into(),
            vendor_device: Some((0x1002, 0x73bf)),
            driver: None,
            compute: true,
        };
        let s = a.summary();
        assert!(s.contains("Radeon RX 6800"), "{s}");
        assert!(s.contains("vulkan"), "{s}");
        assert!(s.contains("1002:73bf"), "{s}");
        assert!(s.contains("compute"), "{s}");
    }
}
