use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use querent_core::complete::engine::Engine;
use querent_core::complete::provider::DefaultProviders;
use querent_core::complete::rank::DefaultRanker;
use querent_core::complete::types::Completion;
use querent_core::dialect::DialectKind;
use querent_core::schema;

use crate::types;

#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct LspProvider {
    engines: Rc<RefCell<HashMap<String, Engine<DefaultProviders, DefaultRanker>>>>,
}

#[wasm_bindgen]
impl LspProvider {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[wasm_bindgen(js_name = "getIntrospectionQueries")]
    pub fn get_introspection_queries(
        &self, dialect: types::JsDialectKind,
    ) -> Result<types::JsIntrospectionQueries, JsValue> {
        let kind: DialectKind = dialect.try_into_rs()?;
        let queries: types::IntrospectionQueries = kind.into();
        types::JsIntrospectionQueries::try_from_rs(&queries)
    }

    #[wasm_bindgen(js_name = "setDialect")]
    pub fn set_dialect(&self, uri: String, dialect: types::JsDialectKind) {
        let kind = dialect.try_into_rs().unwrap_or_default();
        let spec = kind.spec();
        if let Some(engine) = self.engines.borrow_mut().get_mut(&uri) {
            if engine.spec.name != spec.name {
                engine.spec = spec;
            }
        } else {
            self.engines
                .borrow_mut()
                .insert(uri, Engine::new(spec, schema::Cache::default()));
        }
    }

    #[wasm_bindgen(js_name = "setSchema")]
    pub fn set_schema(&self, uri: String, schema: types::JsCache) -> Result<(), JsValue> {
        let schema = schema.try_into_rs()?;
        if let Some(engine) = self.engines.borrow_mut().get_mut(&uri) {
            engine.schema = schema;
        } else {
            self.engines
                .borrow_mut()
                .insert(uri, Engine::new(DialectKind::default().spec(), schema));
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = "removePage")]
    pub fn remove(&self, uri: String) {
        self.engines.borrow_mut().remove(&uri);
    }
}

impl querent_lsp::CompletionProvider for LspProvider {
    fn complete(&mut self, uri: String, doc: &querent_core::doc::Content) -> Vec<Completion> {
        if let Some(engine) = self.engines.borrow_mut().get_mut(&uri) {
            engine.complete(doc).items
        } else {
            vec![]
        }
    }
}
