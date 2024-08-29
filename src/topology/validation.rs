use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use crate::components::name::ComponentName;
use crate::components::output::{ComponentOutput, NamedOutput};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("the same input {input} is being used by multiple components {targets:?}")]
    MultipleUseOfInput {
        input: ComponentOutput<'static>,
        targets: Vec<ComponentName>,
    },
    #[error("a circular dependency has been detected in path {path:?}")]
    CircularDependency { path: HashSet<ComponentName> },
}

type Relations<'a> = HashMap<ComponentOutput<'a>, Vec<&'a ComponentName>>;
type Nodes<'a> = HashMap<&'a ComponentName, HashSet<&'a NamedOutput>>;

#[derive(Default)]
struct Path<'a>(HashSet<&'a ComponentName>);

impl<'a> Path<'a> {
    fn into_inner(self) -> HashSet<ComponentName> {
        self.0.into_iter().cloned().collect()
    }
}

impl<'a> Path<'a> {
    pub fn with(&self, next: &'a ComponentName) -> (bool, Self) {
        let mut inner = self.0.clone();
        let is_new = inner.insert(next);
        (is_new, Self(inner))
    }
}

fn relations_to_nodes<'a>(relations: &'a Relations<'a>) -> Nodes {
    relations.keys().fold(HashMap::new(), |mut res, output| {
        res.entry(output.name.as_ref())
            .or_default()
            .insert(output.output.as_ref());
        res
    })
}

fn find_circular_dependencies<'a>(
    errors: &mut Vec<ValidationError>,
    relations: &Relations,
    nodes: &Nodes,
    current: &Path<'a>,
    output: &'a ComponentOutput<'a>,
) {
    let Some(targets) = relations.get(output) else {
        // This is an output without reader, it's ok.
        return;
    };
    for target in targets {
        let (is_new, new_path) = current.with(target);
        if is_new {
            if let Some(named_outputs) = nodes.get(target) {
                for named_output in named_outputs {
                    let output = ComponentOutput {
                        name: Cow::Borrowed(target),
                        output: Cow::Borrowed(named_output),
                    };
                    find_circular_dependencies(errors, relations, nodes, current, &output);
                }
            }
        } else {
            errors.push(ValidationError::CircularDependency {
                path: new_path.into_inner(),
            });
        }
    }
}

impl super::Config {
    fn many_relations<'a>(&'a self) -> Relations {
        self.sinks
            .iter()
            .flat_map(|(name, sink)| {
                sink.inputs
                    .iter()
                    .map(move |input| (input.to_borrowed(), name))
            })
            .chain(self.transforms.iter().flat_map(|(name, transform)| {
                transform
                    .inputs
                    .iter()
                    .map(move |input| (input.to_borrowed(), name))
            }))
            .fold(Relations::new(), |mut res, (input, target)| {
                res.entry(input).or_default().push(target);
                res
            })
    }

    fn sources_outputs(&self) -> impl Iterator<Item = ComponentOutput<'_>> {
        self.sources.iter().flat_map(|(name, source)| {
            source
                .outputs()
                .into_iter()
                .map(move |output| ComponentOutput {
                    name: Cow::Borrowed(name),
                    output: Cow::Owned(output),
                })
        })
    }

    fn check_circular_dependencies(&self, errors: &mut Vec<ValidationError>) {
        let relations = self.many_relations();
        let nodes = relations_to_nodes(&relations);
        let root = Path::default();
        for output in self.sources_outputs() {
            find_circular_dependencies(errors, &relations, &nodes, &root, &output);
        }
    }

    fn check_input_single_use<'a>(&'a self, errors: &mut Vec<ValidationError>) {
        let input_to_target = self.many_relations();
        for (input, targets) in input_to_target
            .into_iter()
            .filter(|(_, targets)| targets.len() > 1)
        {
            errors.push(ValidationError::MultipleUseOfInput {
                input: input.to_owned(),
                targets: targets.into_iter().map(|v| v.clone()).collect(),
            })
        }
    }

    pub fn validate(self) -> Result<Self, Vec<ValidationError>> {
        let mut errors = Vec::new();
        self.check_input_single_use(&mut errors);
        self.check_circular_dependencies(&mut errors);

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }
}
