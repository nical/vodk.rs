use std::f32::consts::PI;
use std::cmp::{ Ordering };
use std::mem::swap;

use tesselation::{ vertex_id, VertexId };
use tesselation::path::*;
use tesselation::vertex_builder::{ VertexBufferBuilder, Range, };
use tesselation::math_utils::{
    is_below, tangent,
    segment_intersection,line_intersection, line_horizontal_intersection,
};
use tesselation::basic_shapes::{ tesselate_quad };

use vodk_math::{ Vec2 };

#[cfg(test)]
use vodk_math::{ vec2 };
#[cfg(test)]
use tesselation::vertex_builder::{ VertexBuffers, simple_vertex_builder, };
#[cfg(test)]
use tesselation::path_builder::PathBuilder;

struct Event {
    pub current: Vertex,
    pub next: Vertex,
    pub previous: Vertex,
}

pub struct Intersection {
    point: Vertex,
    a_down: Vertex,
    b_down: Vertex,
}

#[derive(Copy, Clone)]
pub struct SortedEventSlice<'l> {
    pub events: &'l[PathVertexId]
}

/// Contains the events of a path and provide access to them, sorted from top to bottom
/// (assuming y points downwards).
pub struct EventVector {
    events: Vec<PathVertexId>
}

impl EventVector {
    pub fn new() -> EventVector {
        EventVector { events: Vec::new() }
    }

    pub fn from_path(
        path: PathSlice,
    ) -> EventVector {
        let mut ev = EventVector {
            events: Vec::with_capacity(path.vertices().len())
        };
        ev.set_path(path);
        return ev;
    }

    pub fn set_path(&mut self,
        path: PathSlice,
    ) {
        self.events.clear();
        for sub_path in path.path_ids() {
            self.events.extend(path.vertex_ids(sub_path));
        }
        debug_assert_eq!(self.events.len(), path.vertices().len());

        self.events.sort_by(|a, b| {
            let va = path.vertex(*a).position;
            let vb = path.vertex(*b).position;
            if va.y > vb.y { return Ordering::Greater; }
            if va.y < vb.y { return Ordering::Less; }
            if va.x > vb.x { return Ordering::Greater; }
            if va.x < vb.x { return Ordering::Less; }
            return Ordering::Equal;
        });
    }

    pub fn as_slice(&self) -> SortedEventSlice {
        SortedEventSlice { events: &self.events[..] }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Side { Left, Right }

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn is_left(self) -> bool { self == Side::Left }

    pub fn is_right(self) -> bool { self == Side::Right }
}

#[derive(Copy, Clone, Debug)]
struct SpanEdge {
    upper: Vertex,
    lower: Vertex,
    merge: bool,
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: Vec2,
    id: PathVertexId,
}

#[derive(Copy, Clone, Debug)]
struct Edge {
    upper: Vertex,
    lower: Vertex,
}

struct Span {
    left: SpanEdge,
    right: SpanEdge,
    monotone_tesselator: MonotoneTesselator,
}

impl Span {
    fn begin(l: SpanEdge, r: SpanEdge) -> Span {
        Span {
            left: l, right: r,
            monotone_tesselator: MonotoneTesselator::begin(l.upper.position, l.upper.id.vertex_id),
        }
    }

    fn vertex(&mut self,
        vertex: Vertex,
        next_vertex: Vertex,
        side: Side
    ) {
        self.set_upper_vertex(vertex, side);
        self.set_lower_vertex(next_vertex, side);
    }

    fn merge_vertex(&mut self, vertex: Vertex, side: Side) {
        self.set_upper_vertex(vertex, side);
        self.set_no_lower_vertex(side);
    }

    fn set_upper_vertex(&mut self, vertex: Vertex, side: Side) {
        self.mut_edge(side).upper = vertex;
        self.monotone_tesselator.vertex(vertex.position, vertex.id.vertex_id, side);
    }

    fn set_lower_vertex(&mut self, vertex: Vertex, side: Side) {
        let mut edge = self.mut_edge(side);

        edge.lower = vertex;
        edge.merge = false;
    }

    fn set_no_lower_vertex(&mut self, side: Side) {
        self.mut_edge(side).merge = true;
    }

    fn mut_edge(&mut self, side: Side) -> &mut SpanEdge {
        return match side {
            Side::Left => { &mut self.left }
            Side::Right => { &mut self.right }
        };
    }

    fn end(&mut self, pos: Vec2, id: PathVertexId) {
        self.monotone_tesselator.end(pos, id.vertex_id);
    }
}

struct SweepLine {
    spans: Vec<Span>
}

pub struct Tesselator<'l, Output: VertexBufferBuilder<Vec2>+'l> {
    path: PathSlice<'l>,
    sweep_line: SweepLine,
    intersections: Vec<Intersection>,
    next_new_vertex: PathVertexId,
    output: &'l mut Output,
}

impl<'l, Output: VertexBufferBuilder<Vec2>> Tesselator<'l, Output> {
    pub fn new(path: PathSlice<'l>, output: &'l mut Output) -> Tesselator<'l, Output> {
        Tesselator {
            path: path,
            sweep_line: SweepLine {
                spans: Vec::with_capacity(16),
            },
            intersections: Vec::new(),
            next_new_vertex: PathVertexId {
                vertex_id: vertex_id(path.num_vertices() as u16),
                path_id: path_id(path.num_sub_paths() as u16),
            },
            output: output,
        }
    }

    pub fn log_sl(&self) {

            print!("\n|  sl: ");
            for span in &self.sweep_line.spans {
                print!("| {:?}   {:?} |  ", span.left.upper.id.vertex_id.handle, span.right.upper.id.vertex_id.handle,);
            }
            print!("\n|    : ");
            for span in &self.sweep_line.spans {
                print!("| {:?}   {:?} |  ", span.left.lower.id.vertex_id.handle, span.right.lower.id.vertex_id.handle,);
            }
            println!("");
        }

    pub fn tesselate(&mut self, sorted_events: SortedEventSlice<'l>) {

        for &e in sorted_events.events {
            let p = self.path.previous(e);
            let n = self.path.next(e);
            let evt = Event {
                current: Vertex { position: self.path.vertex(e).position, id: e },
                previous: Vertex { position: self.path.vertex(p).position, id: p },
                next: Vertex { position: self.path.vertex(n).position, id: n },
            };

            while !self.intersections.is_empty() {
                if is_below(evt.current.position, self.intersections[0].point.position) {
                    let inter = self.intersections.remove(0);
                    self.on_intersection_event(&inter);
                } else {
                    break;
                }
            }

            self.on_event(&evt);
        }
    }

    fn on_event(&mut self, event: &Event) {
        //println!("\n -- on event {:?}", event.current.id.vertex_id.handle);

        let below_prev = is_below(event.current.position, event.previous.position);
        let below_next = is_below(event.current.position, event.next.position);

        if below_prev && below_next {
            return self.on_down_event(event.current);
        }

        if !below_prev && !below_next {
            return self.on_up_event(event);
        }

        let next_below_prev = is_below(event.next.position, event.current.position);

        return self.on_regular_event(
            event.current,
            if next_below_prev { event.next } else { event.previous }
        );
    }

    fn find_span_and_side(&self, vertex: PathVertexId) -> (usize, Side) {
        let mut span_index = 0;
        for span in &self.sweep_line.spans {
            if !span.left.merge && span.left.lower.id == vertex {
                return (span_index, Side::Left);
            }
            if !span.right.merge && span.right.lower.id == vertex {
                return (span_index, Side::Right);
            }
            span_index += 1;
        }

        unreachable!();
    }

    // (edge below, span id, side)
    fn on_regular_event(&mut self, current: Vertex, next: Vertex) {

        let (span_index, side) = self.find_span_and_side(current.id);

        match side {
            Side::Left => { self.on_left_event(span_index, current, next); }
            Side::Right => { self.on_right_event(span_index, current, next); }
        }
    }

    fn on_left_event(&mut self, span_index: usize, current: Vertex, next: Vertex) {
        //println!(" ++++++ Left event {}", current.id.vertex_id.handle);

        if self.sweep_line.spans[span_index].right.merge {
            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            // ll x   <-- current vertex
            //     \r
            self.sweep_line.spans[span_index+1].set_lower_vertex(current, Side::Left);
            self.end_span(span_index, current);
        }

        self.insert_edge(span_index, Side::Left, current, next);
    }

    fn on_left_event_no_intersection(&mut self, span_index: usize, current: Vertex, next: Vertex) {
        //println!(" ++++++ Left event {}", current.id.vertex_id.handle);

        if self.sweep_line.spans[span_index].right.merge {
            //     \ /
            //  \   x   <-- merge vertex
            //   \ :
            // ll x   <-- current vertex
            //     \r
            self.sweep_line.spans[span_index+1].set_lower_vertex(current, Side::Left);
            self.end_span(span_index, current);
        }

        self.insert_edge_no_intersection(span_index, Side::Left, current, next);
    }

    fn on_right_event(&mut self, span_index: usize, current: Vertex, next: Vertex) {
        //println!(" ++++++ Right event {}", current.id.vertex_id.handle);

        self.insert_edge(span_index, Side::Right, current, next);
    }

    fn on_right_event_no_intersection(&mut self, span_index: usize, current: Vertex, next: Vertex) {
        //println!(" ++++++ Right event {}", current.id.vertex_id.handle);

        self.insert_edge_no_intersection(span_index, Side::Right, current, next);
    }

    fn find_span_up(&self, vertex: Vertex) -> (usize, bool) {
        let (x, y) = vertex.position.tuple();
        let mut span_index = 0;
        for span in &self.sweep_line.spans {
            if !span.left.merge {
                let lx = line_horizontal_intersection(
                    span.left.upper.position,
                    span.left.lower.position,
                    y
                );
                if lx > x {
                    return (span_index, false); // outside
                }
            }
            if !span.right.merge {
                let rx = line_horizontal_intersection(
                    span.right.upper.position,
                    span.right.lower.position,
                    y
                );
                if rx > x {
                    return (span_index, true); // inside
                }
            }
            span_index += 1;
        }

        return (span_index, false);
    }

    fn on_up_event(&mut self, event: &Event) {
        let (span_index, is_inside) = self.find_span_up(event.current);

        let mut l = SpanEdge {
            upper: event.current,
            lower: event.previous,
            merge: false,
        };
        let mut r = SpanEdge {
            upper: event.current,
            lower: event.next,
            merge: false,
        };

        let angle = (event.previous.position - event.current.position).directed_angle(
            event.next.position - event.current.position
        );

        if angle < PI {
            swap(&mut l, &mut r);
        }

        if is_inside {
            self.on_split_event(event, span_index, &l, &r)
        } else {
            //println!(" ++++++ Start event {}", event.current.id.vertex_id.handle);
            // Start event.

            let non_existant_index = self.sweep_line.spans.len();
            self.check_intersections(non_existant_index, Side::Left, l.upper, l.lower);
            self.check_intersections(non_existant_index, Side::Right, r.upper, r.lower);
            self.sweep_line.spans.insert(span_index, Span::begin(l, r));
        }
    }

    fn on_split_event(&mut self, event: &Event, span_index: usize, l: &SpanEdge, r: &SpanEdge) {
        //println!(" ++++++ Split event {}", event.current.id.vertex_id.handle);

        // look whether the span shares a merge vertex with the previous one
        if self.sweep_line.spans[span_index].left.merge {
            let left_span = span_index-1;
            let right_span = span_index;
            //            \ /
            //             x   <-- merge vertex
            //  left_span  :  righ_span
            //             x   <-- current split vertex
            //           l/ \r
            self.sweep_line.spans[left_span].vertex(event.current, l.lower, Side::Right);
            self.sweep_line.spans[right_span].vertex(event.current, r.lower, Side::Left);
        } else {
            //      /
            //     x
            //    / :r2
            // ll/   x   <-- current split vertex
            //     l/ \r
            let ll = self.sweep_line.spans[span_index].left;
            let r2 = SpanEdge {
                upper: ll.upper,
                lower: event.current,
                merge: false,
            };

            self.sweep_line.spans.insert(span_index, Span::begin(ll, r2));

            self.insert_edge(span_index, Side::Right, event.current, l.lower);
            self.insert_edge(span_index+1, Side::Left, event.current, r.lower);
        }
    }

    fn on_down_event(&mut self, vertex: Vertex) {
        let (span_index, side) = self.find_span_and_side(vertex.id);

        assert!(span_index < self.sweep_line.spans.len());

        if side.is_left() {
            self.on_end_event(vertex, span_index);
        } else {
            self.on_merge_event(vertex, span_index);
        }
    }

    fn on_end_event(&mut self, vertex: Vertex, span_index: usize) {
        //println!(" ++++++ End event {}", vertex.id.vertex_id.handle);

        if self.sweep_line.spans[span_index].right.merge {
            //   \ /
            //  \ x   <-- merge vertex
            //   \:/
            //    x   <-- current vertex
            self.end_span(span_index, vertex);
        }

        self.end_span(span_index, vertex);
    }

    fn on_merge_event(&mut self, vertex: Vertex, span_index: usize) {
        //println!(" ++++++ Merge event {}", event.current.vertex_id.handle);
        assert!(span_index < self.sweep_line.spans.len()-1);

        if self.sweep_line.spans[span_index].right.merge {
            //     / \ /
            //  \ / .-x    <-- merge vertex
            //   x-'      <-- current merge vertex
            self.sweep_line.spans[span_index+2].set_lower_vertex(vertex, Side::Left);
            self.end_span(span_index+1, vertex);
        }

        debug_assert!(self.sweep_line.spans[span_index+1].left.lower.id == vertex.id);

        self.sweep_line.spans[span_index].merge_vertex(vertex, Side::Right);
        self.sweep_line.spans[span_index+1].merge_vertex(vertex, Side::Left);
    }

    fn insert_edge(&mut self,
        span_index: usize, side: Side,
        up: Vertex, down: Vertex
    ) {
        self.check_intersections(span_index, side, up, down);
        self.sweep_line.spans[span_index].vertex(up, down, side);
    }

    fn insert_edge_no_intersection(&mut self,
        span_index: usize, side: Side,
        up: Vertex, down: Vertex
    ) {
        self.sweep_line.spans[span_index].vertex(up, down, side);
    }

    fn check_intersections(&mut self,
        span_index: usize, side: Side,
        up: Vertex, down: Vertex
    ) {
        for idx in 0..self.sweep_line.spans.len() {
            if idx != span_index || side.is_right() {
                let left = self.sweep_line.spans[idx].left.clone();
                self.test_intersection(&left, up, down);
            }
            if idx != span_index || side.is_left() {
                let right = self.sweep_line.spans[idx].right.clone();
                self.test_intersection(&right, up, down);
            }
        }
    }

    fn test_intersection(
        &mut self, edge: &SpanEdge,
        up: Vertex, down: Vertex
    ) {
        if !edge.merge
        && edge.lower.id != up.id
        && edge.lower.id != down.id {
            if let Some(intersection) = segment_intersection(
                edge.upper.position, edge.lower.position,
                up.position, down.position,
            ) {
                let mut evt = Intersection {
                    point: self.new_vertex(intersection),
                    a_down: down,
                    b_down: edge.lower,
                };

                //println!(" -- found an intersection at {:?}", intersection);
                //println!("    | {:?}->{:?} x {:?}->{:?}",
                //    edge.upper.position, edge.lower.position,
                //    up.position, down.position,
                //);
                //println!("    | new vertex: {:?}", evt.point.id.vertex_id.handle);

                if intersection.directed_angle2(evt.b_down.position, evt.a_down.position) > PI {
                    swap(&mut evt.a_down, &mut evt.b_down);
                }

                self.intersections.push(evt);

                self.intersections.sort_by(|a, b| {
                    let va = a.point.position;
                    let vb = b.point.position;
                    if va.y > vb.y { return Ordering::Greater; }
                    if va.y < vb.y { return Ordering::Less; }
                    if va.x > vb.x { return Ordering::Greater; }
                    if va.x < vb.x { return Ordering::Less; }
                    return Ordering::Equal;
                });
            }
        }
    }


    fn new_vertex(&mut self, pos: Vec2) -> Vertex {
        let ret = Vertex {
            id: self.next_new_vertex,
            position: pos,
        };

        self.next_new_vertex = PathVertexId {
            vertex_id: vertex_id(self.next_new_vertex.vertex_id.handle + 1),
            path_id: self.next_new_vertex.path_id,
        };

        self.output.push_vertex(pos);

        return ret;
    }

    fn on_intersection_event(&mut self, intersection: &Intersection) {

        //println!("\n ------ Intersection evt {:?} {:?}", intersection.point.id, intersection.point.position);

        for idx in 0..self.sweep_line.spans.len() {
            let (l, r) = {
                let span = &self.sweep_line.spans[idx];
                (span.left.lower.id, span.right.lower.id)
            };

            //println!("      {}: searching SL for intersection span: {} {} inter: {} {}", idx,
            //    l.unwrap().vertex_id.handle, r.unwrap().vertex_id.handle,
            //    intersection.a_down.id.vertex_id.handle,
            //    intersection.b_down.id.vertex_id.handle
            //);

            if r == intersection.b_down.id {
                // left + right events
                //println!(" -- L/R intersection");
                self.on_right_event_no_intersection(idx, intersection.point, intersection.a_down);
                self.on_left_event_no_intersection(idx+1, intersection.point, intersection.b_down);
                return;
            }

            if l == intersection.b_down.id {
                //println!(" -- U/D intersection");
                // up + down events
                self.on_end_event(intersection.point, idx);

                let l = SpanEdge {
                    upper: intersection.point,
                    lower: intersection.a_down,
                    merge: false,
                };
                let r = SpanEdge {
                    upper: intersection.point,
                    lower: intersection.b_down,
                    merge: false,
                };
                self.sweep_line.spans.insert(idx, Span::begin(l, r));

                return;
            }
        }

        unreachable!();
    }

    fn end_span(&mut self, span_index: usize, vertex: Vertex) {
        self.sweep_line.spans[span_index].end(vertex.position, vertex.id);
        self.sweep_line.spans[span_index].monotone_tesselator.flush(self.output);
        self.sweep_line.spans.remove(span_index);
    }
}

/// helper class that generates a triangulation from a sequence of vertices describing a monotone
/// polygon.
struct MonotoneTesselator {
    stack: Vec<MonotoneVertex>,
    previous: MonotoneVertex,
    triangles: Vec<(u16, u16, u16)>,
}

#[derive(Copy, Clone, Debug)]
struct MonotoneVertex {
    pos: Vec2,
    id: VertexId,
    side: Side,
}

impl MonotoneTesselator {
    pub fn begin(pos: Vec2, id: VertexId) -> MonotoneTesselator {
        let first = MonotoneVertex { pos: pos, id: id, side: Side::Left };

        let mut tess = MonotoneTesselator {
            stack: Vec::with_capacity(16),
            triangles: Vec::with_capacity(128),
            previous: first,
        };

        tess.stack.push(first);

        return tess;
    }

    pub fn vertex(&mut self, pos: Vec2, id: VertexId, side: Side) {
        let current = MonotoneVertex{ pos: pos, id: id, side: side };
        let right_side = current.side == Side::Right;

        assert!(is_below(current.pos, self.previous.pos));
        assert!(!self.stack.is_empty());

        let changed_side = current.side != self.previous.side;

        if changed_side {
            for i in 0..(self.stack.len() - 1) {
                let mut a = self.stack[i];
                let mut b = self.stack[i+1];

                if right_side {
                    swap(&mut a, &mut b);
                }

                self.push_triangle(&a, &b, &current);
            }
            self.stack.clear();
            self.stack.push(self.previous);
        } else {
            let mut last_popped = self.stack.pop();
            while !self.stack.is_empty() {
                let mut a = last_popped.unwrap();
                let mut b = *self.stack.last().unwrap();

                if right_side {
                    swap(&mut a, &mut b);
                }

                if (current.pos - b.pos).directed_angle(a.pos - b.pos) <= PI {
                    self.push_triangle(&a, &b, &current);
                    last_popped = self.stack.pop();
                } else {
                    break;
                }
            }
            if let Some(item) = last_popped {
                self.stack.push(item);
            }
        }

        self.stack.push(current);
        self.previous = current;
    }

    pub fn end(&mut self, pos: Vec2, id: VertexId) {
        let side = self.previous.side.opposite();
        self.vertex(pos, id, side);
        self.stack.clear();
    }

    fn push_triangle(&mut self, a: &MonotoneVertex, b: &MonotoneVertex, c: &MonotoneVertex) {
        //println!(" #### triangle {} {} {}", a.id.handle, b.id.handle, c.id.handle);

        if (c.pos - b.pos).directed_angle(a.pos - b.pos) <= PI {
            self.triangles.push((a.id.handle, b.id.handle, c.id.handle));
        } else {
            self.triangles.push((b.id.handle, a.id.handle, c.id.handle));
        }
    }

    fn flush<Output: VertexBufferBuilder<Vec2>>(&mut self, output: &mut Output) {
        for &(a, b, c) in &self.triangles {
            output.push_indices(a, b, c);
        }
        self.triangles.clear();
    }
}

#[test]
fn test_monotone_tess() {
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.end(vec2(1.0, 2.0), vertex_id(2));
        assert_eq!(tess.triangles.len(), 1);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(-1.5, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.end(vec2(0.0, 5.0), vertex_id(5));
        assert_eq!(tess.triangles.len(), 4);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(1.0, 1.0), vertex_id(1), Side::Right);
        tess.vertex(vec2(3.0, 2.0), vertex_id(2), Side::Right);
        tess.vertex(vec2(1.0, 3.0), vertex_id(3), Side::Right);
        tess.vertex(vec2(1.0, 4.0), vertex_id(4), Side::Right);
        tess.vertex(vec2(4.0, 5.0), vertex_id(5), Side::Right);
        tess.end(vec2(0.0, 6.0), vertex_id(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
    {
        let mut tess = MonotoneTesselator::begin(vec2(0.0, 0.0), vertex_id(0));
        tess.vertex(vec2(-1.0, 1.0), vertex_id(1), Side::Left);
        tess.vertex(vec2(-3.0, 2.0), vertex_id(2), Side::Left);
        tess.vertex(vec2(-1.0, 3.0), vertex_id(3), Side::Left);
        tess.vertex(vec2(-1.0, 4.0), vertex_id(4), Side::Left);
        tess.vertex(vec2(-4.0, 5.0), vertex_id(5), Side::Left);
        tess.end(vec2(0.0, 6.0), vertex_id(6));
        assert_eq!(tess.triangles.len(), 5);
    }
    println!(" ------------ ");
}

pub struct TesselatorOptions {
    pub vertex_aa: bool,
}

impl TesselatorOptions {
    pub fn new() -> TesselatorOptions {
        TesselatorOptions { vertex_aa: false, }
    }
}

pub fn tesselate_path_fill<'l, Output: VertexBufferBuilder<Vec2>>(
    path: PathSlice<'l>,
    options: &TesselatorOptions,
    output: &mut Output
) -> Result<(), ()> {
    if options.vertex_aa {
        println!("[tesselate_path_fill] Vertex anti-aliasing not implemented");
    }

    println!("[tesselate_path_fill] path {:>?}", path.num_vertices());

    output.begin_geometry();

    for v in path.vertices().as_slice() {
        output.push_vertex(v.position);
    }

    let events = EventVector::from_path(path);
    let mut tess = Tesselator::new(path, output);
    tess.tesselate(events.as_slice());

    return Ok(());
}

pub fn tesselate_path_stroke<Output: VertexBufferBuilder<Vec2>>(
    path: PathSlice,
    thickness: f32,
    output: &mut Output
) -> (Range, Range) {
    output.begin_geometry();
    for p in path.path_ids() {
        tesselate_sub_path_stroke(path.sub_path(p), thickness, output);
    }
    return output.end_geometry();
}

pub fn tesselate_sub_path_stroke<Output: VertexBufferBuilder<Vec2>>(
    path: SubPathSlice,
    thickness: f32,
    output: &mut Output
) {
    let is_closed = path.info().is_closed;

    let first = path.first();
    let mut i = first;
    let mut done = false;

    let mut prev_v1: Vec2 = Default::default();
    let mut prev_v2: Vec2 = Default::default();
    loop {
        let mut p1 = path.vertex(i).position;
        let mut p2 = path.vertex(i).position;

        let extruded = extrude_along_tangent(path, i, thickness, is_closed);
        let d = extruded - p1;

        p1 = p1 + (d * 0.5);
        p2 = p2 - (d * 0.5);

        if i != first || done {
            // TODO: should reuse vertices instead of tesselating quads
            tesselate_quad(prev_v1, prev_v2, p2, p1, output);
        }

        if done {
            break;
        }

        prev_v1 = p1;
        prev_v2 = p2;

        i = path.next(i);

        if i == first {
            if !is_closed {
                break;
            }
            done = true;
        }
    }
}

pub fn extrude_along_tangent(
    path: SubPathSlice,
    i: VertexId,
    amount: f32,
    is_closed: bool
) -> Vec2 {

    let px = path.vertex(i).position;
    let _next = path.next_vertex(i).position;
    let _prev = path.previous_vertex(i).position;

    let prev = if i == path.first() && !is_closed { px + px - _next } else { _prev };
    let next = if i == path.last() && !is_closed { px + px - _prev } else { _next };

    let n1 = tangent(px - prev) * amount;
    let n2 = tangent(next - px) * amount;

    // Segment P1-->PX
    let pn1  = prev + n1; // prev extruded along the tangent n1
    let pn1x = px + n1; // px extruded along the tangent n1
    // Segment PX-->P2
    let pn2  = next + n2;
    let pn2x = px + n2;

    let inter = match line_intersection(pn1, pn1x, pn2x, pn2) {
        Some(v) => { v }
        None => {
            if (n1 - n2).square_length() < 0.000001 {
                pn1x
            } else {
                // TODO: the angle is very narrow, use rounded corner instead
                //panic!("Not implemented yet");
                println!("!! narrow angle at {:?} {:?} {:?} | {:?} {:?} {:?}",
                    px, n1.directed_angle(n2), px.directed_angle2(prev, next),
                    prev.tuple(), px.tuple(), next.tuple(),
                );
                px + (px - prev) * amount / (px - prev).length()
            }
        }
    };
    return inter;
}

#[cfg(test)]
fn test_path(path: PathSlice, expected_triangle_count: Option<usize>) {
    let mut buffers: VertexBuffers<Vec2> = VertexBuffers::new();
    {
        let mut vertex_builder = simple_vertex_builder(&mut buffers);
        let events = EventVector::from_path(path);
        let mut tess = Tesselator::new(path, &mut vertex_builder);
        tess.tesselate(events.as_slice());
    }
    if let Some(num_triangles) = expected_triangle_count {
        assert_eq!(buffers.indices.len()/3, num_triangles);
    }
}

#[cfg(test)]
fn test_path_with_rotations(path: Path, step: f32, expected_triangle_count: Option<usize>) {
    let mut angle = 0.0;

    while angle < PI * 2.0 {
        let mut tranformed_path = path.clone();
        let cos = angle.cos();
        let sin = angle.sin();
        for v in tranformed_path.mut_vertices() {
            let (x, y) = (v.position.x, v.position.y);
            v.position.x = x*cos + y*sin;
            v.position.y = y*cos - x*sin;
        }
        println!("\n\n ==================== angle = {}", angle);
        test_path(tranformed_path.as_slice(), expected_triangle_count);

        angle += step;
    }
}

#[test]
fn test_tesselator_simple_monotone() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(-1.0, 1.0));
    path.line_to(vec2(-3.0, 2.0));
    path.line_to(vec2(-1.0, 3.0));
    path.line_to(vec2(-4.0, 5.0));
    path.line_to(vec2( 0.0, 6.0));
    path.close();

    let path = path.finish();
    test_path(path.as_slice(), Some(4));
}

#[test]
fn test_tesselator_simple_split() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(1.0, 2.0));
    path.line_to(vec2(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, Some(3));
}

#[test]
fn test_tesselator_simple_merge_split() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(1.0, 2.0));
    path.line_to(vec2(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, Some(4));
}

#[test]
fn test_tesselator_simple_aligned() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(2.0, 2.0));
    path.line_to(vec2(1.0, 2.0));
    path.line_to(vec2(0.0, 2.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, Some(6));
}

#[test]
fn test_tesselator_simple_1() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 1.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(1.0, 3.0));
    path.line_to(vec2(0.5, 4.0));
    path.line_to(vec2(0.0, 3.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, Some(4));
}

#[test]
fn test_tesselator_simple_2() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(1.0, 0.0));
    path.line_to(vec2(2.0, 0.0));
    path.line_to(vec2(3.0, 0.0));
    path.line_to(vec2(3.0, 1.0));
    path.line_to(vec2(3.0, 2.0));
    path.line_to(vec2(3.0, 3.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(1.0, 3.0));
    path.line_to(vec2(0.0, 3.0));
    path.line_to(vec2(0.0, 2.0));
    path.line_to(vec2(0.0, 1.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, Some(10));
}

#[test]
fn test_tesselator_hole_1() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(-11.0, 5.0));
    path.line_to(vec2(0.0, -5.0));
    path.line_to(vec2(10.0, 5.0));
    path.close();

    path.move_to(vec2(-5.0, 2.0));
    path.line_to(vec2(0.0, -2.0));
    path.line_to(vec2(4.0, 2.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, Some(6));
}

#[test]
fn test_tesselator_degenerate_empty() {
    test_path(Path::new().as_slice(), Some(0));
}

#[test]
fn test_tesselator_degenerate_same_position() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.line_to(vec2(0.0, 0.0));
    path.close();

    test_path_with_rotations(path.finish(), 0.001, None);
}

#[test]
fn test_tesselator_auto_intersection_type1() {
    //  o.___
    //   \   'o
    //    \ /
    //     x  <-- intersection!
    //    / \
    //  o.___\
    //       'o
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(0.0, 2.0));
    path.line_to(vec2(2.0, 3.0));
    path.close();

    let path = path.finish();
    test_path(path.as_slice(), Some(2));
}

#[test]
fn test_tesselator_auto_intersection_type2() {
    //  o
    //  |\   ,o
    //  | \ / |
    //  |  x  | <-- intersection!
    //  | / \ |
    //  o'   \|
    //        o
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(0.0, 0.0));
    path.line_to(vec2(2.0, 3.0));
    path.line_to(vec2(2.0, 1.0));
    path.line_to(vec2(0.0, 2.0));
    path.close();

    let path = path.finish();
    test_path(path.as_slice(), Some(2));
}

#[test]
fn test_tesselator_auto_intersection_multi() {
    //      .
    //  ___/_\___
    //  | /   \ |
    //  |/     \|
    // /|       |\
    // \|       |/
    //  |\     /|
    //  |_\___/_|
    //     \ /
    //      '
    let mut path = PathBuilder::new();
    path.set_flattening(true);
    path.move_to(vec2(20.0, 20.0));
    path.line_to(vec2(60.0, 20.0));
    path.line_to(vec2(60.0, 60.0));
    path.line_to(vec2(20.0, 60.0));
    path.close();

    path.move_to(vec2(40.0, 10.0));
    path.line_to(vec2(70.0, 40.0));
    path.line_to(vec2(40.0, 70.0));
    path.line_to(vec2(10.0, 40.0));
    path.close();

    let path = path.finish();
    test_path(path.as_slice(), Some(8));
}

#[test]
fn test_tesselator_rust_logo() {
    let mut path = PathBuilder::new();
    path.set_flattening(true);

    ::tesselation::rust_logo::build_logo_path(&mut path);

    test_path_with_rotations(path.finish(), 0.011, None);
}
