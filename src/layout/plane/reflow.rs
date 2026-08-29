use smithay::utils::{Logical, Point, Rectangle};

#[derive(Debug, Clone)]
pub(in crate::layout) struct Snapshot<I> {
    pub id: I,
    pub rectangle: Rectangle<f64, Logical>,
}

#[derive(Debug, Clone)]
pub(in crate::layout) struct Instruction<I> {
    pub id: I,
    pub offset: Point<f64, Logical>,
}

#[derive(Debug)]
pub(in crate::layout) struct State<I> {
    previous: Vec<Snapshot<I>>,
}

impl<I> Default for State<I> {
    fn default() -> Self {
        Self {
            previous: Vec::new(),
        }
    }
}

impl<I: Clone + PartialEq> State<I> {
    pub fn update(&mut self, current: &[Snapshot<I>], added: &[I]) -> Vec<Instruction<I>> {
        let instructions = instructions(&self.previous, current)
            .into_iter()
            .filter(|instruction| added.iter().all(|id| id != &instruction.id))
            .collect();
        self.previous = current.to_vec();
        instructions
    }
}

pub(in crate::layout) fn instructions<I: Clone + PartialEq>(
    old: &[Snapshot<I>],
    new: &[Snapshot<I>],
) -> Vec<Instruction<I>> {
    new.iter()
        .filter_map(|target| {
            let previous = old.iter().find(|item| item.id == target.id)?;
            let offset = previous.rectangle.loc - target.rectangle.loc;
            (offset != Point::default()).then(|| Instruction {
                id: target.id.clone(),
                offset,
            })
        })
        .collect()
}
