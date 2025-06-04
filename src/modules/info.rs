/*!
The `Info` module is automatically created when the rack has an `[info]`
section.

##### Note
The `Info` module cannot be created directly.

## Inputs
None

## Outputs
None

## Knobs
None

*/

use bevy::{prelude::*, ecs::system::EntityCommands, utils::HashMap};

use serde::{Deserialize, de};

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent}};

#[derive(Debug, Clone)]
pub struct Info {
    id: Option<usize>,
    name: Option<String>,

    component: Option<Entity>,
    children: Vec<Entity>,

    info: HashMap<String, String>,
}
impl Info {
    pub fn new(info: HashMap<String, String>) -> Self {
        Self {
            id: None,
            name: None,

            component: None,
            children: vec![],

            info,
        }
    }
}
#[typetag::deserialize]
impl Module for Info {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, tfc: (TextFont, TextColor)) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn((
                Node {
                    position_type: PositionType::Relative,
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(150.0),
                    height: Val::Px(180.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("Info\n"),
                };
                self.children.push(
                    parent.spawn((
                        Text::new(name),
                        tfc.0.clone(),
                        tfc.1.clone(),
                        ModuleTextComponent,
                    )).with_children(|p| {
                        for t in self.info.iter()
                            .map(|(k, v)| format!("{k}: {v}\n"))
                        {
                            p.spawn((
                                TextSpan::new(t),
                                tfc.0.clone(),
                                tfc.1.clone(),
                            ));
                        }
                    }).id()
                );
            });
            self.component = Some(component.id());
        });
    }
    fn exit(&mut self) {
        self.id = None;
        self.component = None;
        self.children = vec![];
    }

    fn id(&self) -> Option<usize> {
        self.id
    }
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        0
    }
    fn outputs(&self) -> usize {
        0
    }
    fn knobs(&self) -> usize {
        0
    }

    fn step(&mut self, _time: f64, _st: StepType, _ins: &[f32]) -> Vec<f32> {
        vec![]
    }
}
impl<'de> Deserialize<'de> for Info {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(de::Error::custom("Cannot use the Info module directly, instead define an [info] section"))
    }
}
