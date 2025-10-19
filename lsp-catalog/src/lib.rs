use std::collections::HashMap;

use querent_core::complete::Engine;

pub struct LspEngines {
    engines: HashMap<String, Engine>,
}
// mod catalog;
// mod remote;

// pub use catalog::*;
// pub use remote::*;
