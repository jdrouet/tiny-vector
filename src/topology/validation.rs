use std::collections::{HashMap, HashSet, VecDeque};

use super::{Config, WithInputs};
use crate::components::name::ComponentName;
use crate::components::output::{ComponentOutput, NamedOutput};

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("the same input {input} is being used by multiple components {targets:?}")]
    MultipleUseOfInput {
        input: ComponentOutput<'static>,
        targets: HashSet<ComponentName>,
    },
    #[error("unable to find the specified input {input}")]
    InputNotFound { input: ComponentOutput<'static> },
    #[error("unable to find output {output} in the component {name}")]
    OutputNotFound {
        name: ComponentName,
        output: NamedOutput,
    },
    #[error("component {name} should have at least one input")]
    NoInput { name: ComponentName },
    // In theory, this is never reachable considering an output can only be used once.
    #[error("circular dependency detected in the path {path:?} with {name}")]
    CircularDependency {
        path: HashSet<ComponentName>,
        name: ComponentName,
    },
    #[error("component {name} is not part of a route that goes from a source to a sink")]
    OrphanComponent { name: ComponentName },
}

type RelationMap<'a> = HashMap<ComponentOutput<'a>, HashSet<&'a ComponentName>>;

enum Node<'a> {
    Source {
        outputs: HashSet<NamedOutput>,
    },
    Transform {
        inputs: &'a HashSet<ComponentOutput<'a>>,
        outputs: HashSet<NamedOutput>,
    },
    Sink {
        inputs: &'a HashSet<ComponentOutput<'a>>,
    },
}

impl<'a> Node<'a> {
    fn source(element: &'a crate::sources::Config) -> Self {
        Self::Source {
            outputs: element.outputs(),
        }
    }

    fn transform(element: &'a WithInputs<crate::transforms::Config>) -> Self {
        Self::Transform {
            inputs: &element.inputs,
            outputs: element.inner.outputs(),
        }
    }

    fn sink(element: &'a WithInputs<crate::sinks::Config>) -> Self {
        Self::Sink {
            inputs: &element.inputs,
        }
    }

    fn inputs(&self) -> Option<&HashSet<ComponentOutput<'a>>> {
        match &self {
            Self::Sink { inputs } => Some(inputs),
            Self::Transform { inputs, outputs: _ } => Some(inputs),
            Self::Source { outputs: _ } => None,
        }
    }

    fn outputs(&self) -> Option<&HashSet<NamedOutput>> {
        match &self {
            Self::Sink { inputs: _ } => None,
            Self::Transform { inputs: _, outputs } => Some(outputs),
            Self::Source { outputs } => Some(outputs),
        }
    }
}

struct Graph<'a> {
    config: &'a super::Config,
    nodes: HashMap<&'a ComponentName, Node<'a>>,
    relations: RelationMap<'a>,
}

impl<'a> Graph<'a> {
    fn build(config: &'a Config) -> Self {
        let nodes = config
            .sources
            .iter()
            .map(|(name, source)| (name, Node::source(source)))
            .chain(
                config
                    .transforms
                    .iter()
                    .map(|(name, transform)| (name, Node::transform(transform))),
            )
            .chain(
                config
                    .sinks
                    .iter()
                    .map(|(name, sink)| (name, Node::sink(sink))),
            )
            .collect();
        let relations = config
            .sinks
            .iter()
            .flat_map(|(name, sink)| sink.inputs.iter().map(move |input| (input, name)))
            .fold(RelationMap::new(), |mut res, (input, name)| {
                res.entry(input.to_borrowed()).or_default().insert(name);
                res
            });
        Self {
            config,
            nodes,
            relations,
        }
    }

    fn check_multiple_use_of_input(&self, errors: &mut Vec<ValidationError>) {
        for (output, targets) in self.relations.iter() {
            if targets.len() > 1 {
                errors.push(ValidationError::MultipleUseOfInput {
                    input: output.to_owned(),
                    targets: HashSet::from_iter(targets.iter().map(|item| (*item).clone())),
                });
            }
        }
    }

    fn traverse_backward(&'a self, errors: &mut Vec<ValidationError>) {
        let mut used_components = HashSet::new();
        let mut stack = VecDeque::<(ComponentOutput<'a>, HashSet<ComponentName>)>::new();
        for (name, sink) in self.config.sinks.iter() {
            if sink.inputs.is_empty() {
                errors.push(ValidationError::NoInput { name: name.clone() });
                continue;
            }
            for input in sink.inputs.iter() {
                stack.push_back((input.to_borrowed(), HashSet::from_iter([name.clone()])));
            }
        }
        while let Some((output, mut path)) = stack.pop_front() {
            let Some(node) = self.nodes.get(output.name.as_ref()) else {
                errors.push(ValidationError::InputNotFound {
                    input: output.to_owned(),
                });
                continue;
            };
            let Some(outputs) = node.outputs() else {
                errors.push(ValidationError::OutputNotFound {
                    name: output.to_owned_name(),
                    output: output.to_owned_output(),
                });
                continue;
            };
            if !outputs.contains(output.output.as_ref()) {
                errors.push(ValidationError::OutputNotFound {
                    name: output.to_owned_name(),
                    output: output.to_owned_output(),
                });
                continue;
            }
            if let Some(inputs) = node.inputs() {
                if inputs.is_empty() {
                    errors.push(ValidationError::NoInput {
                        name: output.to_owned_name(),
                    });
                    continue;
                }
                if !path.insert(output.to_owned_name()) {
                    errors.push(ValidationError::CircularDependency {
                        name: output.to_owned_name(),
                        path,
                    });
                    continue;
                }
                for input in inputs.iter() {
                    stack.push_back((input.to_borrowed(), path.clone()));
                }
            } else {
                used_components.extend(path);
                used_components.insert(output.to_owned_name());
            }
        }
        println!("existing nodes: {:?}", self.nodes.keys());
        println!("used nodes: {:?}", used_components);
        let existing_nodes: HashSet<ComponentName> =
            HashSet::from_iter(self.nodes.keys().map(|v| (*v).clone()));
        for name in existing_nodes.difference(&used_components) {
            errors.push(ValidationError::OrphanComponent { name: name.clone() });
        }
    }
}

impl Config {
    pub fn validate(self) -> Result<Self, Vec<ValidationError>> {
        let mut errors = Vec::new();

        let graph = Graph::build(&self);
        graph.check_multiple_use_of_input(&mut errors);
        graph.traverse_backward(&mut errors);

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }
}
