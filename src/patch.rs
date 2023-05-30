use std::collections::{HashMap, HashSet};

use bevy::prelude::Component;
use serde::Deserialize;

use crate::modules::ModuleKey;

#[derive(Deserialize, Debug, Clone)]
pub struct Patches(HashMap<ModuleKey, HashSet<ModuleKey>>);
impl Patches {
    pub fn iter(&self) -> impl Iterator<Item = (&'_ ModuleKey, &'_ ModuleKey)> {
        self.0.iter()
            .flat_map(|(output, inputs)| {
                inputs.iter()
                    .map(move |input| (output, input))
            })
    }
}

impl<'a> FromIterator<(&'a ModuleKey, &'a ModuleKey)> for Patches {
    fn from_iter<T: IntoIterator<Item = (&'a ModuleKey, &'a ModuleKey)>>(iter: T) -> Self {
        let mut patches = Patches(HashMap::new());
        for (k, v) in iter.into_iter() {
            let patch = patches.0.entry(*k).or_default();
            patch.insert(*v);
        }
        patches
    }
}

impl<'a> IntoIterator for &'a Patches {
    type Item = (&'a ModuleKey, &'a ModuleKey);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Component, Debug, Clone)]
pub struct PatchComponent;