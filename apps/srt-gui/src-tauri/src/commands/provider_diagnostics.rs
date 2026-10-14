use serde::Serialize;
use srt_translate::{TranslationDiagnostic, TranslatorPool};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
struct ProviderRetry {
    tier: usize,
    endpoint: usize,
    #[serde(flatten)]
    diagnostic: TranslationDiagnostic,
}

/// Desktop composition owns event delivery; engines remain independent of Tauri.
pub(super) fn observe_pool(mut pool: TranslatorPool, app: &AppHandle) -> TranslatorPool {
    for (tier_index, tier) in pool.iter_mut().enumerate() {
        for (endpoint_index, entry) in tier.iter_mut().enumerate() {
            let app = app.clone();
            entry.translator = entry
                .translator
                .clone()
                .with_diagnostics(move |diagnostic| {
                    let _ = app.emit(
                        "provider-retry",
                        ProviderRetry {
                            tier: tier_index + 1,
                            endpoint: endpoint_index + 1,
                            diagnostic,
                        },
                    );
                });
        }
    }
    pool
}
