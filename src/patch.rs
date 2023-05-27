use std::collections::{HashMap, HashSet};

use bevy::prelude::Component;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Patches(HashMap<String, HashSet<String>>);
impl Patches {
    pub fn iter(&self) -> impl Iterator<Item = (&'_ str, &'_ str)> {
        self.0.iter()
            .flat_map(|(output, inputs)| {
                inputs.iter()
                    .map(move |input| (output.as_str(), input.as_str()))
            })
    }
}

impl<'a> FromIterator<(&'a str, &'a str)> for Patches {
    fn from_iter<T: IntoIterator<Item = (&'a str, &'a str)>>(iter: T) -> Self {
        let mut patches = Patches(HashMap::new());
        for (k, v) in iter.into_iter() {
            let patch = patches.0.entry(k.to_string()).or_default();
            patch.insert(v.to_string());
        }
        patches
    }
}

impl<'a> IntoIterator for &'a Patches {
    type Item = (&'a str, &'a str);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Component, Debug, Clone)]
pub struct PatchComponent;