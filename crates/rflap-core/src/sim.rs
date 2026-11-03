//! Generic simulation engine (JFLAP `AutomatonSimulator`), reusable by every machine.

use std::collections::{HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;

/// A nondeterministic machine seen as a configuration graph.
pub trait Machine {
    type Config: Clone + Eq + Hash + Debug;

    /// Starting configurations for an input string.
    fn initial_configs(&self, input: &str) -> Vec<Self::Config>;
    /// Successors of a configuration, in transition order.
    fn step(&self, config: &Self::Config) -> Vec<Self::Config>;
    /// Whether the configuration accepts.
    fn is_accepting(&self, config: &Self::Config) -> bool;
}

/// Index of a node in a run arena.
pub type NodeId = usize;

struct Node<C> {
    config: C,
    parent: Option<NodeId>,
}

fn path_to<C: Clone>(nodes: &[Node<C>], node: NodeId) -> Vec<C> {
    let mut path = Vec::new();
    let mut at = Some(node);
    while let Some(n) = at {
        path.push(nodes[n].config.clone());
        at = nodes[n].parent;
    }
    path.reverse();
    path
}

/// Step-by-step run: every `step` replaces each frontier configuration by its successors.
pub struct StepRun<'a, M: Machine> {
    machine: &'a M,
    nodes: Vec<Node<M::Config>>,
    frontier: Vec<NodeId>,
    accepted: Vec<NodeId>,
    rejected: Vec<NodeId>,
}

impl<'a, M: Machine> StepRun<'a, M> {
    pub fn new(machine: &'a M, input: &str) -> Self {
        let mut run = StepRun {
            machine,
            nodes: Vec::new(),
            frontier: Vec::new(),
            accepted: Vec::new(),
            rejected: Vec::new(),
        };
        for config in machine.initial_configs(input) {
            let id = run.push(config, None);
            run.frontier.push(id);
        }
        run
    }

    fn push(&mut self, config: M::Config, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.len();
        if self.machine.is_accepting(&config) {
            self.accepted.push(id);
        }
        self.nodes.push(Node { config, parent });
        id
    }

    /// Advances one step; configurations without successors that do not accept are
    /// recorded as rejected. Returns whether the frontier is still non-empty.
    pub fn step(&mut self) -> bool {
        let mut next = Vec::new();
        for node in std::mem::take(&mut self.frontier) {
            let successors = self.machine.step(&self.nodes[node].config);
            if successors.is_empty() && !self.accepted.contains(&node) {
                self.rejected.push(node);
            }
            for config in successors {
                next.push(self.push(config, Some(node)));
            }
        }
        self.frontier = next;
        !self.frontier.is_empty()
    }

    /// Current configurations.
    pub fn frontier(&self) -> &[NodeId] {
        &self.frontier
    }

    /// Every accepting configuration reached so far.
    pub fn accepted(&self) -> &[NodeId] {
        &self.accepted
    }

    /// Dead-end configurations that do not accept.
    pub fn rejected(&self) -> &[NodeId] {
        &self.rejected
    }

    pub fn config(&self, node: NodeId) -> &M::Config {
        &self.nodes[node].config
    }

    /// Configurations from the root to `node`.
    pub fn trace(&self, node: NodeId) -> Vec<M::Config> {
        path_to(&self.nodes, node)
    }
}

/// Result of a bounded run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome<C> {
    Accepted {
        trace: Vec<C>,
    },
    Rejected,
    /// The configuration budget ran out before the search finished.
    Undecided,
}

impl<C> RunOutcome<C> {
    pub fn is_accepted(&self) -> bool {
        matches!(self, RunOutcome::Accepted { .. })
    }
}

/// Breadth-first search for an accepting configuration. Configurations already seen are
/// not expanded again, so λ-cycles terminate; at most `max_configs` are created.
pub fn fast_run<M: Machine>(machine: &M, input: &str, max_configs: usize) -> RunOutcome<M::Config> {
    let mut nodes: Vec<Node<M::Config>> = Vec::new();
    let mut seen: HashSet<M::Config> = HashSet::new();
    let mut expand: VecDeque<NodeId> = VecDeque::new();
    let mut pending: VecDeque<(M::Config, Option<NodeId>)> = machine
        .initial_configs(input)
        .into_iter()
        .map(|c| (c, None))
        .collect();
    loop {
        while let Some((config, parent)) = pending.pop_front() {
            if !seen.insert(config.clone()) {
                continue;
            }
            if nodes.len() >= max_configs {
                return RunOutcome::Undecided;
            }
            let accepting = machine.is_accepting(&config);
            nodes.push(Node { config, parent });
            let id = nodes.len() - 1;
            if accepting {
                return RunOutcome::Accepted {
                    trace: path_to(&nodes, id),
                };
            }
            expand.push_back(id);
        }
        let Some(node) = expand.pop_front() else {
            return RunOutcome::Rejected;
        };
        pending.extend(
            machine
                .step(&nodes[node].config)
                .into_iter()
                .map(|c| (c, Some(node))),
        );
    }
}

/// `fast_run` over several inputs.
pub fn multiple_run<M, I, S>(
    machine: &M,
    inputs: I,
    max_configs: usize,
) -> Vec<(String, RunOutcome<M::Config>)>
where
    M: Machine,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    inputs
        .into_iter()
        .map(|input| {
            let input = input.as_ref();
            (input.to_string(), fast_run(machine, input, max_configs))
        })
        .collect()
}
