//! Placement provenance and conservative fallbacks when driver process records
//! are inaccessible. Inference is explicitly not proof of GPU ownership.
use crate::{gpu::GpuStats, model_detect::DetectedModel};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Placement {
    #[default]
    Direct,
    Inferred(String),
    Configured(String),
    Unavailable(String),
}

impl Placement {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Inferred(_) => "inferred",
            Self::Configured(_) => "configured",
            Self::Unavailable(_) => "unknown",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Direct => None,
            Self::Inferred(reason) | Self::Configured(reason) | Self::Unavailable(reason) => {
                Some(reason)
            }
        }
    }
}

/// Environment mappings precede CLI mappings, so explicit flags win per port.
/// Both forms use PORT=amd|nvidia|intel|pci:DOMAIN:BUS:DEVICE.FUNCTION.
pub fn mappings(env: Option<&str>, cli: &[String]) -> Vec<String> {
    env.into_iter()
        .flat_map(|v| v.split(','))
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_owned())
        .chain(cli.iter().cloned())
        .collect()
}

pub fn configure(
    models: &mut [DetectedModel],
    inventory: &[GpuStats],
    mappings: &[String],
    parent: impl Fn(u32) -> Option<u32>,
) -> Result<(), String> {
    // Validate everything before changing any models.
    let resolved: Vec<_> = mappings
        .iter()
        .map(|mapping| {
            let (port, selector) = mapping.split_once('=').ok_or_else(|| {
                format!("invalid --server-gpu '{mapping}': expected PORT=SELECTOR")
            })?;
            let port = port
                .parse::<u16>()
                .ok()
                .filter(|p| *p != 0)
                .ok_or_else(|| format!("invalid server port in '{mapping}'"))?;
            let selector = selector.trim().to_ascii_lowercase();
            let pci = selector.strip_prefix("pci:").unwrap_or(&selector);
            let matches: Vec<_> = inventory
                .iter()
                .filter(|g| match selector.as_str() {
                    "amd" => g.backend == "amd",
                    "nvidia" => matches!(g.backend, "nvml" | "smi"),
                    "intel" => g.backend == "xpu",
                    _ => g
                        .pci_address
                        .as_deref()
                        .is_some_and(|p| p.eq_ignore_ascii_case(pci)),
                })
                .collect();
            if matches.len() != 1 {
                return Err(format!(
                    "--server-gpu '{mapping}' matches {} GPUs; use a unique PCI address",
                    matches.len()
                ));
            }
            Ok((port, matches[0].index, mapping.clone()))
        })
        .collect::<Result<_, String>>()?;
    for (port, gpu, mapping) in resolved {
        let targets: HashSet<_> = models
            .iter()
            .filter(|m| m.port == Some(port))
            .map(|m| m.pid)
            .collect();
        for model in models.iter_mut() {
            if model.port == Some(port)
                || (model.pid != 0 && has_ancestor(model.pid, &targets, &parent))
            {
                // Driver memory from a different device cannot survive an override.
                if model.gpu_indices != [gpu] {
                    model.mem_used_mb = 0;
                }
                model.gpu_indices = vec![gpu];
                model.placement = Placement::Configured(format!("{mapping} (owner supplied)"));
            }
        }
    }
    Ok(())
}

fn has_ancestor(
    mut pid: u32,
    targets: &HashSet<u32>,
    parent: &impl Fn(u32) -> Option<u32>,
) -> bool {
    let mut seen = HashSet::new();
    while seen.insert(pid) {
        let Some(ppid) = parent(pid).filter(|p| *p > 1) else {
            break;
        };
        if targets.contains(&ppid) {
            return true;
        }
        pid = ppid;
        if seen.len() >= 64 {
            break;
        }
    }
    false
}

#[cfg(target_os = "linux")]
pub fn denied_reason(model: &DetectedModel) -> Option<String> {
    if model.pid == 0 {
        return None;
    }
    let path = format!("/proc/{}/fd", model.pid);
    std::fs::read_dir(&path)
        .err()
        .filter(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
        .map(|_| format!("permission denied reading {path}; needs server-user/root access"))
}

#[cfg(not(target_os = "linux"))]
pub fn denied_reason(_model: &DetectedModel) -> Option<String> {
    None
}

pub fn infer_denied(
    models: &mut [DetectedModel],
    inventory: &[GpuStats],
    denied: impl Fn(&DetectedModel) -> Option<String>,
) {
    let unknown: Vec<_> = models
        .iter_mut()
        .enumerate()
        .filter(|(_, m)| m.gpu_indices.is_empty() && m.n_gpu_layers != Some(0))
        .map(|(i, m)| {
            let reason = denied(m);
            m.placement = Placement::Unavailable(
                reason
                    .clone()
                    .unwrap_or_else(|| "no readable GPU ownership evidence".into()),
            );
            (i, reason)
        })
        .collect();
    // Multiple unplaced servers (even one that is readable) defeat elimination.
    let [(index, Some(reason))] = unknown.as_slice() else {
        return;
    };
    let claimed: HashSet<_> = models
        .iter()
        .flat_map(|m| m.gpu_indices.iter().copied())
        .collect();
    let candidates: Vec<_> = inventory
        .iter()
        .filter(|g| !claimed.contains(&g.index) && g.mem_used_mb > 0 && g.telemetry_error.is_none())
        .collect();
    let [gpu] = candidates.as_slice() else { return };
    let model = &mut models[*index];
    model.gpu_indices = vec![gpu.index];
    // Total device VRAM is not a measured per-process allocation.
    model.mem_used_mb = 0;
    model.placement = Placement::Inferred(format!("sole unclaimed GPU with VRAM used; {reason}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(pid: u32, port: u16, gpu: Option<u32>) -> DetectedModel {
        let mut m = crate::demo::demo_models(8192, 1).remove(0);
        m.pid = pid;
        m.port = Some(port);
        m.gpu_indices = gpu.into_iter().collect();
        m.mem_used_mb = 0;
        m.placement = Placement::Direct;
        m
    }

    fn inventory() -> Vec<GpuStats> {
        vec![
            GpuStats {
                index: 0,
                backend: "nvml",
                mem_used_mb: 23412,
                pci_address: Some("0000:02:00.0".into()),
                ..Default::default()
            },
            GpuStats {
                index: 1,
                backend: "amd",
                mem_used_mb: 22228,
                pci_address: Some("0000:01:00.0".into()),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn restricted_proc_eliminates_only_one_unclaimed_gpu_and_marks_inference() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/mixed-gpu/restricted-proc.json"))
                .unwrap();
        let llama_pid = fixture["llama_pid"].as_u64().unwrap() as u32;
        let mut models = vec![
            model(
                fixture["strata_pid"].as_u64().unwrap() as u32,
                fixture["strata_port"].as_u64().unwrap() as u16,
                Some(0),
            ),
            model(
                llama_pid,
                fixture["llama_port"].as_u64().unwrap() as u16,
                None,
            ),
        ];
        assert_eq!(fixture["fd_read_error"], "PermissionDenied");
        let denied = |m: &DetectedModel| {
            (m.pid == llama_pid).then(|| fixture["denied_reason"].as_str().unwrap().to_owned())
        };
        infer_denied(&mut models, &inventory(), denied);
        assert_eq!(models[0].placement, Placement::Direct);
        assert_eq!(models[1].gpu_indices, vec![1]);
        assert_eq!(models[1].placement.label(), "inferred");
        assert!(models[1]
            .placement
            .reason()
            .unwrap()
            .contains("permission denied reading /proc/1721/fd"));
        assert_eq!(models[1].mem_used_mb, 0); // not falsely measured process memory
    }

    #[test]
    fn ambiguity_no_vram_and_readable_unknown_never_infer() {
        let denied = |_: &DetectedModel| Some("permission denied reading /proc/1721/fd".into());
        for (mut models, mut gpus) in [
            (vec![model(1721, 34885, None)], inventory()), // two candidates
            (
                vec![
                    model(1997, 8098, Some(0)),
                    model(1721, 34885, None),
                    model(3000, 8081, None),
                ],
                inventory(),
            ),
            (
                vec![model(1997, 8098, Some(0)), model(1721, 34885, None)],
                inventory(),
            ),
        ] {
            if models.len() == 2 {
                gpus[1].mem_used_mb = 0;
            }
            infer_denied(&mut models, &gpus, denied);
            assert!(models
                .iter()
                .filter(|m| m.pid != 1997)
                .all(|m| m.gpu_indices.is_empty()));
            assert!(models
                .iter()
                .filter(|m| m.pid != 1997)
                .all(|m| m.placement.label() == "unknown"));
        }
        let mut models = vec![model(1997, 8098, Some(0)), model(1721, 34885, None)];
        infer_denied(&mut models, &inventory(), |_| None);
        assert!(models[1].gpu_indices.is_empty());
        let mut gpus = inventory();
        gpus[1].telemetry_error = Some("driver unavailable".into());
        infer_denied(&mut models, &gpus, denied);
        assert!(models[1].gpu_indices.is_empty());
    }

    #[test]
    fn direct_evidence_is_not_replaced_by_elimination() {
        let mut models = vec![model(1721, 34885, Some(1))];
        infer_denied(&mut models, &inventory(), |_| Some("denied".into()));
        assert_eq!(models[0].gpu_indices, vec![1]);
        assert_eq!(models[0].placement, Placement::Direct);
    }

    #[test]
    fn configuration_overrides_direct_and_reaches_router_children_cli_wins() {
        let mut models = vec![
            model(1688, 8080, Some(0)),
            model(1721, 34885, Some(0)),
            model(1997, 8098, Some(0)),
        ];
        let maps = mappings(Some("8080=nvidia"), &["8080=pci:0000:01:00.0".into()]);
        configure(&mut models, &inventory(), &maps, |pid| {
            (pid == 1721).then_some(1688)
        })
        .unwrap();
        for m in &models[..2] {
            assert_eq!(m.gpu_indices, vec![1]);
            assert_eq!(m.placement.label(), "configured");
            assert!(m
                .placement
                .reason()
                .unwrap()
                .contains("8080=pci:0000:01:00.0"));
        }
        assert_eq!(models[2].placement, Placement::Direct);
    }

    #[test]
    fn invalid_or_ambiguous_mapping_fails_without_partial_changes() {
        for mapping in [
            "not-a-map",
            "0=amd",
            "65536=amd",
            "8080=unknown",
            "8080=pci:0000:99:00.0",
        ] {
            let mut models = vec![model(1721, 8080, Some(0))];
            assert!(configure(
                &mut models,
                &inventory(),
                &["8080=amd".into(), mapping.into()],
                |_| None
            )
            .is_err());
            assert_eq!(models[0].gpu_indices, vec![0]);
        }
        let mut gpus = inventory();
        gpus.push(GpuStats {
            index: 2,
            backend: "amd",
            ..Default::default()
        });
        assert!(configure(
            &mut [model(1721, 8080, None)],
            &gpus,
            &["8080=amd".into()],
            |_| None
        )
        .is_err());
    }
}
