use std::collections::HashMap;

use crate::components::name::ComponentName;
use crate::components::output::ComponentOutput;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("the same input {input} is being used by multiple components {targets:?}")]
    MultipleUseOfInput {
        input: ComponentOutput<'static>,
        targets: Vec<ComponentName>,
    },
}

type Relations<'a> = HashMap<ComponentOutput<'a>, Vec<&'a ComponentName>>;

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

        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }
}
