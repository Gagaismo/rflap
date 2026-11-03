//! Graph layout algorithms (JFLAP `automata.graph.layout`).
//!
//! The algorithms are ports of JFLAP 7.1's `CircleLayoutAlgorithm`, `GEMLayoutAlgorithm`,
//! `RandomLayoutAlgorithm`, `SpiralLayoutAlgorithm`, `TreeLayoutAlgorithm` and
//! `TwoCircleLayoutAlgorithm`, including their `VertexChain`/`CircleChain` helpers. They work on
//! the undirected (tree: directed) graph induced by the transitions and are deterministic: the
//! only randomness comes from a small seeded PRNG, and states are visited in id order.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::automaton::{Automaton, Label, Point, StateId};
use crate::render::geometry::STATE_RADIUS;

/// Layout algorithm (JFLAP `LayoutAlgorithmFactory` constants).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    Circle,
    Gem,
    Random,
    Spiral,
    /// Hierarchical tree following the direction of the transitions (`TREE_HIERARCHY`).
    Tree,
    /// Tree ordered by vertex degree, ignoring direction (`TREE_DEGREE`).
    #[serde(rename = "tree-degree", alias = "tree-by-degree")]
    TreeByDegree,
    TwoCircle,
    /// Initial state on the far left, the rest in columns by BFS distance along the
    /// transitions (not part of JFLAP).
    Layered,
}

impl Algorithm {
    pub const ALL: [Algorithm; 8] = [
        Algorithm::Circle,
        Algorithm::Gem,
        Algorithm::Random,
        Algorithm::Spiral,
        Algorithm::Tree,
        Algorithm::TreeByDegree,
        Algorithm::TwoCircle,
        Algorithm::Layered,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Algorithm::Circle => "circle",
            Algorithm::Gem => "gem",
            Algorithm::Random => "random",
            Algorithm::Spiral => "spiral",
            Algorithm::Tree => "tree",
            Algorithm::TreeByDegree => "tree-degree",
            Algorithm::TwoCircle => "two-circle",
            Algorithm::Layered => "layered",
        }
    }
}

impl FromStr for Algorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Algorithm::ALL
            .into_iter()
            .find(|a| a.name() == s.to_ascii_lowercase())
            .ok_or_else(|| format!("unknown layout algorithm `{s}`"))
    }
}

/// Parameters shared by all algorithms (JFLAP `LayoutAlgorithm` fields plus a seed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutOptions {
    /// Size of the box the states are fitted in.
    pub width: f64,
    pub height: f64,
    /// JFLAP `vertexDim`: also the margin kept around the box.
    pub vertex_width: f64,
    pub vertex_height: f64,
    /// JFLAP `vertexBuffer` (the JFLAP GUI passes 50 for FA, 65 Mealy, 80 PDA, 80 per tape for TM).
    pub vertex_buffer: f64,
    /// Final pass: no two states end closer than this (not part of JFLAP).
    pub min_distance: f64,
    /// Seed of the PRNG used by `Random` and `Gem`.
    pub seed: u64,
    /// States the algorithm does not reposition (JFLAP's "set" of unmovable vertices); the
    /// final fit into the box still translates every state, fixed ones included, like JFLAP.
    pub fixed: BTreeSet<StateId>,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions {
            width: 900.0,
            height: 900.0,
            vertex_width: 30.0,
            vertex_height: 30.0,
            vertex_buffer: 30.0,
            min_distance: 2.0 * STATE_RADIUS + 2.0,
            seed: 1,
            fixed: BTreeSet::new(),
        }
    }
}

/// Moves the states of `automaton` according to `algorithm`; coordinates end up integral like
/// JFLAP's `moveAutomatonStates`.
pub fn apply<L: Label>(
    automaton: &mut Automaton<L>,
    algorithm: Algorithm,
    options: &LayoutOptions,
) {
    for (id, point) in compute(automaton, algorithm, options) {
        if let Some(state) = automaton.state_mut(id) {
            state.point = point;
        }
    }
}

/// The positions `apply` would assign, without touching the automaton.
pub fn compute<L: Label>(
    automaton: &Automaton<L>,
    algorithm: Algorithm,
    options: &LayoutOptions,
) -> BTreeMap<StateId, Point> {
    let mut g = Graph::new(
        automaton,
        matches!(algorithm, Algorithm::Tree | Algorithm::Layered),
    );
    let movable: Vec<usize> = (0..g.len())
        .filter(|&i| !options.fixed.contains(&g.ids[i]))
        .collect();
    if !movable.is_empty() {
        let mut rng = Rng::new(options.seed);
        match algorithm {
            Algorithm::Circle => circle(&mut g, &movable, options),
            Algorithm::Gem => gem(&mut g, &movable, &mut rng, options),
            Algorithm::Random => random(&mut g, &movable, &mut rng, options),
            Algorithm::Spiral => spiral(&mut g, &movable, options),
            Algorithm::Tree | Algorithm::TreeByDegree => {
                tree(&mut g, &movable, algorithm == Algorithm::Tree, options)
            }
            Algorithm::TwoCircle => two_circle(&mut g, &movable, options),
            Algorithm::Layered => layered(&mut g, &movable, options),
        }
        separate(&mut g, options);
    }
    g.ids
        .iter()
        .zip(&g.pos)
        .map(|(&id, &(x, y))| {
            (
                id,
                Point {
                    x: x.trunc(),
                    y: y.trunc(),
                },
            )
        })
        .collect()
}

/// SplitMix64: tiny, seedable and identical on every platform.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `0..n` (`n > 0`).
    fn next_below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// JFLAP `AutomatonGraph` / `AutomatonDirectedGraph`: vertices are state indices.
struct Graph {
    ids: Vec<StateId>,
    pos: Vec<(f64, f64)>,
    adj: Vec<BTreeSet<usize>>,
    initial: Option<usize>,
}

impl Graph {
    fn new<L: Label>(a: &Automaton<L>, directed: bool) -> Self {
        let ids: Vec<StateId> = a.states().map(|s| s.id).collect();
        let pos = a.states().map(|s| (s.point.x, s.point.y)).collect();
        let mut adj = vec![BTreeSet::new(); ids.len()];
        for t in a.transitions() {
            let (Ok(f), Ok(to)) = (ids.binary_search(&t.from), ids.binary_search(&t.to)) else {
                continue;
            };
            adj[f].insert(to);
            if !directed {
                adj[to].insert(f);
            }
        }
        let initial = a.initial().and_then(|id| ids.binary_search(&id).ok());
        Graph {
            ids,
            pos,
            adj,
            initial,
        }
    }

    fn len(&self) -> usize {
        self.ids.len()
    }

    fn has_edge(&self, a: usize, b: usize) -> bool {
        self.adj[a].contains(&b)
    }

    fn degree(&self, a: usize) -> usize {
        self.adj[a].len()
    }

    /// JFLAP `AutomatonDirectedGraph.toDegree(v, true)`.
    fn in_degree(&self, v: usize) -> usize {
        (0..self.len())
            .filter(|&u| u != v && self.has_edge(u, v))
            .count()
    }

    fn polar_to_cartesian(&mut self, vertices: &[usize]) {
        for &v in vertices {
            let (r, a) = self.pos[v];
            self.pos[v] = (a.cos() * r, a.sin() * r);
        }
    }
}

/// JFLAP `LayoutAlgorithm.shiftOntoScreen(graph, size, vertexDim, true)`: translates to the
/// origin, shrinks (never grows) each axis to fit the box and adds the margin.
fn shift_onto_screen(g: &mut Graph, o: &LayoutOptions) {
    if g.len() == 0 || o.width == 0.0 || o.height == 0.0 {
        return;
    }
    let min_x = g.pos.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let min_y = g.pos.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_x = g.pos.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let max_y = g.pos.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let ratio_x = (max_x - min_x) / (o.width - 2.0 * o.vertex_width);
    let ratio_y = (max_y - min_y) / (o.height - 2.0 * o.vertex_height);
    for p in &mut g.pos {
        p.0 -= min_x;
        p.1 -= min_y;
        if ratio_x > 1.0 {
            p.0 /= ratio_x;
        }
        if ratio_y > 1.0 {
            p.1 /= ratio_y;
        }
        p.0 += o.vertex_width;
        p.1 += o.vertex_height;
    }
}

/// Diagonal of a vertex plus the buffer: the spacing unit of the chain layouts.
fn spacing(o: &LayoutOptions) -> f64 {
    o.vertex_width.hypot(o.vertex_height) + o.vertex_buffer
}

/// JFLAP `VertexChain` (and `CircleChain` through `layout`): vertices ordered so that
/// neighbours in the graph tend to be neighbours in the chain.
#[derive(Clone, Default)]
struct Chain {
    v: Vec<usize>,
    radius: f64,
}

impl Chain {
    fn len(&self) -> usize {
        self.v.len()
    }

    fn degree_in_chain(&self, g: &Graph, x: usize) -> usize {
        self.v
            .iter()
            .filter(|&&w| g.has_edge(x, w) && w != x)
            .count()
    }

    fn is_edge_to_member(&self, g: &Graph, x: usize) -> bool {
        self.degree_in_chain(g, x) > 0
    }

    fn insert_clamped(&mut self, at: isize, x: usize) {
        let at = at.clamp(0, self.v.len() as isize) as usize;
        self.v.insert(at, x);
    }

    fn orient_sub_chain(&mut self, n: isize, n2: isize, n3: isize, n4: isize, bl: bool) {
        let sub: Vec<usize> = (n3..=n4).map(|i| self.v[i as usize]).collect();
        let n6 = self.len() as isize;
        let n7 = if n > 0 && n >= n3 { n + n3 - n4 - 1 } else { n };
        for x in &sub {
            if let Some(i) = self.v.iter().position(|w| w == x) {
                self.v.remove(i);
            }
        }
        let last = sub.len() - 1;
        for k in 0..sub.len() {
            if bl {
                if n == n6 || n7 == self.len() as isize {
                    let x = if n2 == n3 { sub[last - k] } else { sub[k] };
                    self.v.push(x);
                } else {
                    let x = if n2 == n3 { sub[last - k] } else { sub[k] };
                    self.insert_clamped(n7 + 1, x);
                }
            } else {
                let x = if n2 == n3 { sub[k] } else { sub[last - k] };
                self.insert_clamped(n7, x);
            }
        }
    }

    fn add_vertex(&mut self, g: &Graph, x: usize) {
        let size = self.len();
        for i in 0..size {
            if !g.has_edge(x, self.v[i]) {
                continue;
            }
            let n = if i == size - 1 || !g.has_edge(self.v[i], self.v[i + 1]) {
                i + 1
            } else {
                i
            };
            self.v.insert(n, x);
            for j in i + 2..self.len() {
                if !g.has_edge(x, self.v[j]) || self.degree_in_chain(g, self.v[j]) > 2 {
                    continue;
                }
                let (n_, i_, j_) = (n as isize, i as isize, j as isize);
                if j < self.len() - 1 && g.has_edge(self.v[j], self.v[j + 1]) {
                    self.orient_sub_chain(n_, j_, j_, self.len() as isize - 1, n == i + 1);
                } else {
                    let mut n2 = j_;
                    while n2 > i_ + 2 && g.has_edge(self.v[n2 as usize - 1], self.v[n2 as usize]) {
                        n2 -= 1;
                    }
                    self.orient_sub_chain(n_, j_, n2, j_, n == i + 1);
                }
                return;
            }
            return;
        }
        self.v.push(x);
    }

    /// JFLAP `CircleChain.layout`: polar positions `(radius, angle)` on an arc.
    fn layout(&mut self, g: &mut Graph, o: &LayoutOptions, d: f64, d2: f64, d3: f64) {
        let d4 = spacing(o);
        match self.len() {
            0 => {}
            1 => {
                g.pos[self.v[0]] = if d == 0.0 { (0.0, 0.0) } else { (d + d4, d2) };
            }
            size => {
                let start = d2 - d3 / 2.0;
                let steps = if PI * 2.0 - d3 < 1e-4 { size } else { size - 1 };
                let step = d3 / steps as f64;
                self.radius = (d4 / step).max(d + d4);
                for (i, &x) in self.v.iter().enumerate() {
                    g.pos[x] = (self.radius, start + step * i as f64);
                }
            }
        }
    }
}

/// JFLAP `VertexChain.alignTwoChains`.
fn align_two_chains(c1: &mut Chain, c2: &mut Chain, g: &Graph) {
    for i in 0..c1.len() {
        for j in 0..c2.len() {
            if c1.degree_in_chain(g, c1.v[i]) >= 2
                || c2.degree_in_chain(g, c2.v[j]) >= 2
                || !g.has_edge(c1.v[i], c2.v[j])
            {
                continue;
            }
            let (mut n, mut n2, mut n3, mut n4) = (i, i, j, j);
            while n > 0 && g.has_edge(c1.v[n], c1.v[n - 1]) {
                n -= 1;
            }
            while n2 < c1.len() - 1 && g.has_edge(c1.v[n2], c1.v[n2 + 1]) {
                n2 += 1;
            }
            while n3 > 0 && g.has_edge(c2.v[n3], c2.v[n3 - 1]) {
                n3 -= 1;
            }
            while n4 < c2.len() - 1 && g.has_edge(c2.v[n4], c2.v[n4 + 1]) {
                n4 += 1;
            }
            let (i_, j_) = (i as isize, j as isize);
            let (n_, n2_, n3_, n4_) = (n as isize, n2 as isize, n3 as isize, n4 as isize);
            c1.orient_sub_chain(c1.len() as isize - 1, n_ + n2_ - i_, n_, n2_, true);
            c2.orient_sub_chain(0, n3_ + n4_ - j_, n3_, n4_, false);
            return;
        }
    }
}

// ---------------------------------------------------------------- circle

/// A connected group of the circle layout with its place in the packing.
struct Packed {
    size: (i64, i64),
    upper_left: (f64, f64),
    right: Option<usize>,
    down: Option<usize>,
}

fn circle(g: &mut Graph, movable: &[usize], o: &LayoutOptions) {
    let mut boxes: Vec<Chain> = Vec::new();
    for &v in movable {
        match boxes.iter_mut().find(|b| b.is_edge_to_member(g, v)) {
            Some(b) => b.add_vertex(g, v),
            None => {
                let mut b = Chain::default();
                b.add_vertex(g, v);
                boxes.push(b);
            }
        }
    }
    for n in (1..boxes.len()).rev() {
        'merge: for i in (0..n).rev() {
            for j in 0..boxes[n].len() {
                let w = boxes[n].v[j];
                if boxes[i].is_edge_to_member(g, w) {
                    let moved = boxes.remove(n);
                    for x in moved.v {
                        boxes[i].add_vertex(g, x);
                    }
                    break 'merge;
                }
            }
        }
    }
    let mut packed: Vec<Packed> = Vec::new();
    for k in 0..boxes.len() {
        let mut chain = std::mem::take(&mut boxes[k]);
        chain.layout(g, o, 0.0, PI, PI * 2.0);
        g.polar_to_cartesian(&chain.v);
        let side = |dim: f64| (2.0 * (chain.radius + o.vertex_buffer) + dim) as i64;
        packed.push(Packed {
            size: (side(o.vertex_width), side(o.vertex_height)),
            upper_left: (0.0, 0.0),
            right: None,
            down: None,
        });
        if k != 0 {
            set_upper_left(&mut packed, k, 0);
            let (ux, uy) = packed[k].upper_left;
            for &x in &chain.v {
                g.pos[x].0 += ux;
                g.pos[x].1 += uy;
            }
        }
        boxes[k] = chain;
    }
    shift_onto_screen(g, o);
}

/// JFLAP `Box.setUpperLeft`: places box `me` to the right of or below the packed boxes.
fn set_upper_left(p: &mut [Packed], me: usize, mut other: usize) {
    if p[me].size.1 <= p[other].size.1 {
        p[me].upper_left.0 += p[other].size.0 as f64;
        while let Some(r) = p[other].right {
            other = r;
            p[me].upper_left.0 += p[other].size.0 as f64;
        }
        p[other].right = Some(me);
        return;
    }
    p[me].upper_left.1 += p[other].size.1 as f64;
    match p[other].down {
        None => p[other].down = Some(me),
        Some(d) => set_upper_left(p, me, d),
    }
}

// ---------------------------------------------------------------- spiral

fn spiral(g: &mut Graph, movable: &[usize], o: &LayoutOptions) {
    let mut order = movable.to_vec();
    order.sort_by_key(|&v| std::cmp::Reverse(g.degree(v)));
    let mut chain = Chain::default();
    for &v in &order {
        chain.add_vertex(g, v);
    }
    let step = spacing(o);
    let (mut radius, mut angle) = (0.0_f64, 0.0_f64);
    for &v in &chain.v {
        radius = radius.hypot(step);
        angle += (step / radius).asin();
        g.pos[v] = (radius, angle);
    }
    g.polar_to_cartesian(movable);
    shift_onto_screen(g, o);
}

// ---------------------------------------------------------------- random

fn random(g: &mut Graph, movable: &[usize], rng: &mut Rng, o: &LayoutOptions) {
    let mut chain = Chain::default();
    let mut points: Vec<(f64, f64)> = Vec::new();
    for &v in movable {
        let x = rng.next_f64() * (o.width - o.vertex_buffer * 2.0);
        let y = rng.next_f64() * (o.height - o.vertex_buffer * 2.0);
        points.push((x, y));
        chain.add_vertex(g, v);
    }
    lessen_vertex_overlap(&mut points, o);
    let points = find_correct_point_order(&points);
    for (i, &p) in points.iter().enumerate() {
        g.pos[chain.v[i]] = p;
    }
    shift_onto_screen(g, o);
}

fn lessen_vertex_overlap(points: &mut [(f64, f64)], o: &LayoutOptions) {
    let n = points.len();
    let mut by_x: Vec<usize> = (0..n).collect();
    let mut by_y: Vec<usize> = (0..n).collect();
    by_x.sort_by(|&a, &b| points[b].0.total_cmp(&points[a].0));
    by_y.sort_by(|&a, &b| points[b].1.total_cmp(&points[a].1));
    let w = o.vertex_width + o.vertex_buffer;
    let h = o.vertex_height + o.vertex_buffer;
    for i in 0..n.saturating_sub(1) {
        let dx = points[by_x[i]].0 - points[by_x[i + 1]].0;
        let dy = points[by_x[i]].1 - points[by_x[i + 1]].1;
        if dx < w && dy < h {
            for &k in by_x[..=i].iter() {
                points[k].0 += w - dx;
            }
        }
        let dx = points[by_y[i]].0 - points[by_y[i + 1]].0;
        let dy = points[by_y[i]].1 - points[by_y[i + 1]].1;
        if dx < w && dy < h {
            for &k in by_y[..=i].iter() {
                points[k].1 += h - dy;
            }
        }
    }
}

/// JFLAP `findCorrectPointOrder`: orders the points by a sweep of turning angles.
fn find_correct_point_order(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut current = (0.0, 0.0);
    let mut heading = 0.0_f64;
    let mut ordered = Vec::with_capacity(points.len());
    let mut rest = points.to_vec();
    while !rest.is_empty() {
        let mut best = rest[0];
        let mut best_angle = PI * 2.0 + 1.0;
        for &p in &rest {
            let a = if p.1 != current.1 {
                ((p.0 - current.0) / (p.1 - current.1)).atan()
            } else if p.0 > current.0 {
                PI / 2.0
            } else {
                -PI / 2.0
            };
            let a = (a + PI * 4.0 - heading) % PI;
            if a < best_angle {
                best_angle = a;
                best = p;
            }
        }
        current = best;
        heading = (heading + best_angle) % (PI * 2.0);
        if let Some(i) = rest.iter().position(|&p| p == best) {
            rest.remove(i);
        }
        ordered.push(best);
    }
    ordered
}

// ---------------------------------------------------------------- tree

fn tree(g: &mut Graph, movable: &[usize], hierarchical: bool, o: &LayoutOptions) {
    let mut order = movable.to_vec();
    if hierarchical {
        order.sort_by_key(|&v| g.in_degree(v));
    } else {
        order.sort_by_key(|&v| std::cmp::Reverse(g.degree(v)));
    }
    let mut levels: Vec<Vec<usize>> = vec![Vec::new()];
    while !order.is_empty() {
        levels[0].push(order.remove(0));
        let mut level = 0;
        while level < levels.len() && !order.is_empty() {
            process_children(g, &mut levels, level, &mut order);
            level += 1;
        }
    }
    let mut y = 0.0;
    for vertices in &levels {
        let mut x = -(vertices.len() as f64) * (o.vertex_width + o.vertex_buffer) / 2.0;
        for &v in vertices {
            g.pos[v] = (x, y);
            x += o.vertex_buffer + o.vertex_width;
        }
        y += o.vertex_height + o.vertex_buffer;
    }
    shift_onto_screen(g, o);
}

/// JFLAP `TreeLayoutAlgorithm.Level.processChildren`.
fn process_children(g: &Graph, levels: &mut Vec<Vec<usize>>, level: usize, rest: &mut Vec<usize>) {
    if level + 1 == levels.len() {
        // Created lazily in JFLAP; an empty trailing level adds no row, so drop it afterwards.
        levels.push(Vec::new());
    }
    let parents = levels[level].clone();
    let mut previous: Option<Chain> = None;
    for &parent in &parents {
        let mut chain = Chain::default();
        for j in (0..rest.len()).rev() {
            if g.has_edge(parent, rest[j]) && parent != rest[j] {
                chain.add_vertex(g, rest[j]);
                rest.remove(j);
            }
        }
        if let Some(prev) = previous.as_mut() {
            align_two_chains(prev, &mut chain, g);
            levels[level + 1].extend(prev.v.iter().copied());
        }
        previous = Some(chain);
    }
    if let Some(prev) = previous {
        levels[level + 1].extend(prev.v);
    }
    if levels.last().is_some_and(Vec::is_empty) {
        levels.pop();
    }
}

// ---------------------------------------------------------------- layered

/// Column of every vertex: BFS distance from the initial state (the first state without one);
/// what it does not reach is numbered by BFS from its lowest-index vertex, to the right.
fn layer_columns(g: &Graph) -> Vec<usize> {
    let mut column: Vec<Option<usize>> = vec![None; g.len()];
    let mut next_start = 0;
    let mut start = g.initial.or((g.len() > 0).then_some(0));
    while let Some(root) = start {
        column[root] = Some(next_start);
        let mut queue = std::collections::VecDeque::from([root]);
        while let Some(v) = queue.pop_front() {
            let at = column[v].unwrap_or(next_start);
            for &w in &g.adj[v] {
                if column[w].is_none() {
                    column[w] = Some(at + 1);
                    queue.push_back(w);
                }
            }
        }
        next_start = column.iter().flatten().max().map_or(0, |m| m + 1);
        start = column.iter().position(Option::is_none);
    }
    column.into_iter().flatten().collect()
}

fn layered(g: &mut Graph, movable: &[usize], o: &LayoutOptions) {
    let column = layer_columns(g);
    let step = spacing(o);
    let last = column.iter().max().map_or(0, |m| m + 1);
    for c in 0..last {
        let members: Vec<usize> = (0..g.len()).filter(|&v| column[v] == c).collect();
        let middle = (members.len() as f64 - 1.0) / 2.0;
        for (row, &v) in members.iter().enumerate() {
            if movable.contains(&v) {
                g.pos[v] = (c as f64 * step, (row as f64 - middle) * step);
            }
        }
    }
    shift_onto_screen(g, o);
}

// ---------------------------------------------------------------- two circles

fn two_circle(g: &mut Graph, movable: &[usize], o: &LayoutOptions) {
    let (mut inner, outer) = assign_to_circles(g, movable);
    let mut inner_chain = Chain::default();
    for &v in &inner {
        inner_chain.add_vertex(g, v);
    }
    inner_chain.layout(g, o, 0.0, PI, PI * 2.0);
    inner = inner_chain.v.clone();
    if !outer.is_empty() {
        let mut chains = outer_circle_chains(g, &inner, &outer);
        for i in 0..chains.len() {
            let j = (i + 1) % chains.len();
            if i != j {
                let mut a = std::mem::take(&mut chains[i]);
                let mut b = std::mem::take(&mut chains[j]);
                align_two_chains(&mut a, &mut b, g);
                chains[i] = a;
                chains[j] = b;
            }
        }
        let step = PI * 2.0 / chains.len() as f64;
        for (i, chain) in chains.iter_mut().enumerate() {
            chain.layout(g, o, inner_chain.radius, step * i as f64, step * 4.0 / 5.0);
        }
    }
    g.polar_to_cartesian(movable);
    shift_onto_screen(g, o);
}

/// JFLAP `assignToCircles`: vertices of degree > 2 go inside, then every outer vertex with
/// two or more inner neighbours joins them. (JFLAP skips the element after a removal; so do we.)
fn assign_to_circles(g: &Graph, movable: &[usize]) -> (Vec<usize>, Vec<usize>) {
    let (mut inner, mut outer): (Vec<usize>, Vec<usize>) =
        movable.iter().partition(|&&v| g.degree(v) > 2);
    if inner.is_empty() {
        return (outer, Vec::new());
    }
    loop {
        let mut changed = false;
        let mut i = 0;
        while i < outer.len() {
            let inner_neighbours = inner.iter().filter(|&&w| g.has_edge(outer[i], w)).count();
            if inner_neighbours >= 2 {
                inner.push(outer.remove(i));
                changed = true;
            }
            i += 1;
        }
        if !changed {
            return (inner, outer);
        }
    }
}

/// JFLAP `createOuterCircleChains`: one chain per inner vertex.
fn outer_circle_chains(g: &Graph, inner: &[usize], outer: &[usize]) -> Vec<Chain> {
    let mut chains = vec![Chain::default(); inner.len()];
    let mut assigned: Vec<Option<usize>> = vec![None; outer.len()];
    for (k, &out) in outer.iter().enumerate() {
        for (c, &inn) in inner.iter().enumerate() {
            if g.has_edge(out, inn) {
                chains[c].add_vertex(g, out);
                assigned[k] = Some(c);
            }
        }
    }
    loop {
        let mut changed = false;
        for k in 0..outer.len() {
            if assigned[k].is_some() {
                continue;
            }
            let mut first = None;
            let mut second = None;
            for (c, chain) in chains.iter().enumerate() {
                if chain.is_edge_to_member(g, outer[k]) {
                    if first.is_none() {
                        first = Some(c);
                    } else {
                        second = Some(c);
                    }
                }
            }
            let target = match (first, second) {
                (Some(a), None) => a,
                (Some(a), Some(b)) => {
                    if chains[a].len() < chains[b].len() {
                        a
                    } else {
                        b
                    }
                }
                _ => continue,
            };
            chains[target].add_vertex(g, outer[k]);
            assigned[k] = Some(target);
            changed = true;
        }
        if !changed {
            break;
        }
    }
    let mut smallest = 0;
    for c in 0..chains.len() {
        if chains[smallest].len() > chains[c].len() {
            smallest = c;
        }
    }
    for (k, &out) in outer.iter().enumerate() {
        if assigned[k].is_none() {
            chains[smallest].add_vertex(g, out);
        }
    }
    chains
}

// ---------------------------------------------------------------- GEM

const GEM_TEMPERATURE: f64 = 3.0;
const GEM_OPTIMAL_EDGE_LENGTH: f64 = 100.0;
const GEM_GRAVITY: f64 = 0.0625;

/// JFLAP `GEMLayoutAlgorithm`: starts from the current positions and nudges one random
/// movable vertex at a time with a fixed step length.
fn gem(g: &mut Graph, movable: &[usize], rng: &mut Rng, o: &LayoutOptions) {
    let n = g.len();
    let fixed: Vec<usize> = (0..n).filter(|v| !movable.contains(v)).collect();
    let mut optimal = GEM_OPTIMAL_EDGE_LENGTH;
    let (mut total, mut count) = (0.0, 0);
    for (a, &i) in fixed.iter().enumerate() {
        for &j in &fixed[a + 1..] {
            if g.has_edge(i, j) {
                total += (g.pos[i].0 - g.pos[j].0).hypot(g.pos[i].1 - g.pos[j].1);
                count += 1;
            }
        }
    }
    if count > 0 {
        optimal = total / count as f64;
    }
    let (mut sum_x, mut sum_y) = g
        .pos
        .iter()
        .fold((0.0, 0.0), |(sx, sy), p| (sx + p.0, sy + p.1));
    let mut queue: Vec<usize> = Vec::new();
    for _ in 0..120 * movable.len() {
        if queue.is_empty() {
            queue = movable.to_vec();
        }
        let e = queue.remove(rng.next_below(queue.len()));
        let p = g.pos[e];
        let mut deg = g.degree(e) as f64;
        deg *= 1.0 + deg / 2.0;
        let mut fx = (sum_x / n as f64 - p.0) * GEM_GRAVITY * deg;
        let mut fy = (sum_y / n as f64 - p.1) * GEM_GRAVITY * deg;
        fx += rng.next_f64() * 10.0 - 5.0;
        fy += rng.next_f64() * 10.0 - 5.0;
        for j in 0..n {
            if j == e {
                continue;
            }
            let (dx, dy) = (p.0 - g.pos[j].0, p.1 - g.pos[j].1);
            let dist2 = dx * dx + dy * dy;
            let opt2 = optimal * optimal;
            if dx != 0.0 || dy != 0.0 {
                fx += dx * opt2 / dist2;
                fy += dy * opt2 / dist2;
            }
            if g.has_edge(e, j) {
                fx -= dx * dist2 / (opt2 * deg);
                fy -= dy * dist2 / (opt2 * deg);
            }
        }
        if fx == 0.0 && fy == 0.0 {
            continue;
        }
        let scale = GEM_TEMPERATURE / fx.hypot(fy);
        fx *= scale;
        fy *= scale;
        g.pos[e] = (p.0 + fx, p.1 + fy);
        sum_x += fx;
        sum_y += fy;
    }
    shift_onto_screen(g, o);
}

// ---------------------------------------------------------------- overlap removal

/// Not in JFLAP (which guarantees no minimum distance for GEM, Random and TwoCircle): pushes
/// overlapping pairs apart until every pair is `min_distance` away, staying inside the box.
fn separate(g: &mut Graph, o: &LayoutOptions) {
    let n = g.len();
    let (min_x, max_x) = (
        o.vertex_width,
        (o.width - o.vertex_width).max(o.vertex_width),
    );
    let (min_y, max_y) = (
        o.vertex_height,
        (o.height - o.vertex_height).max(o.vertex_height),
    );
    let fixed: Vec<bool> = g.ids.iter().map(|id| o.fixed.contains(id)).collect();
    for _ in 0..500 {
        let mut moved = false;
        for i in 0..n {
            for j in i + 1..n {
                let (dx, dy) = (g.pos[j].0 - g.pos[i].0, g.pos[j].1 - g.pos[i].1);
                let dist = dx.hypot(dy);
                if dist >= o.min_distance || (fixed[i] && fixed[j]) {
                    continue;
                }
                moved = true;
                let (ux, uy) = if dist > 1e-9 {
                    (dx / dist, dy / dist)
                } else {
                    let a = (i * n + j) as f64 * 2.399_963;
                    (a.cos(), a.sin())
                };
                let push = (o.min_distance - dist) / 2.0 + 0.01;
                let (wi, wj) = match (fixed[i], fixed[j]) {
                    (true, _) => (0.0, 2.0),
                    (_, true) => (2.0, 0.0),
                    _ => (1.0, 1.0),
                };
                g.pos[i].0 = (g.pos[i].0 - ux * push * wi).clamp(min_x, max_x);
                g.pos[i].1 = (g.pos[i].1 - uy * push * wi).clamp(min_y, max_y);
                g.pos[j].0 = (g.pos[j].0 + ux * push * wj).clamp(min_x, max_x);
                g.pos[j].1 = (g.pos[j].1 + uy * push * wj).clamp(min_y, max_y);
            }
        }
        if !moved {
            return;
        }
    }
    move_to_free_slots(g, o, &fixed);
}

/// Last resort when pushing did not converge (for example states squeezed onto one line by
/// the fit): states still in conflict move to the first free cell of a `min_distance` grid.
fn move_to_free_slots(g: &mut Graph, o: &LayoutOptions, fixed: &[bool]) {
    let n = g.len();
    let step = o.min_distance.max(1.0);
    let (min_x, min_y) = (o.vertex_width, o.vertex_height);
    let columns = ((o.width - 2.0 * o.vertex_width) / step).floor().max(0.0) as usize + 1;
    let rows = ((o.height - 2.0 * o.vertex_height) / step).floor().max(0.0) as usize + 1;
    let far = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1) >= o.min_distance;
    for i in 0..n {
        if fixed[i] {
            continue;
        }
        let others = |j: &usize| *j != i && (*j < i || fixed[*j]);
        if (0..n).filter(others).all(|j| far(g.pos[i], g.pos[j])) {
            continue;
        }
        let slot = (0..rows)
            .flat_map(|r| (0..columns).map(move |c| (c, r)))
            .map(|(c, r)| (min_x + c as f64 * step, min_y + r as f64 * step))
            .find(|&p| (0..n).filter(|&j| j != i).all(|j| far(p, g.pos[j])));
        if let Some(p) = slot {
            g.pos[i] = p;
        }
    }
}
