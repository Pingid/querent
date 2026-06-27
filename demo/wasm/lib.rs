use wasm_bindgen::prelude::*;

pub use querent_lsp_wasm::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    tracing::info!("Querent LSP started");
    Ok(())
}
