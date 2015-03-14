// Implementation inspired by Computational Geometry, Algorithms And Applications 3rd edition.
//
// Note that a lot of the code/comments/names in this module assume a coordinate
// system where y pointing downwards

use halfedge::*;
use iterators::{Direction, DirectedEdgeCirculator};
use math::vector::*;
use std::num::Float;
use std::cmp::{Ordering, PartialOrd};
use std::iter::FromIterator;
use std::collections::HashMap;
use std::mem::swap;
use std::fmt::Debug;

#[cfg(test)]
use math::units::world;

#[derive(Debug, Copy, Clone)]
enum VertexType {
    Start,
    End,
    Split,
    Merge,
    Left,
    Right,
}

/// Angle between v1 and v2 (oriented clockwise with y pointing downward)
/// (equivalent to counter-clockwise if y points upward)
/// ex: directed_angle([0,1], [1,0]) = 3/2 Pi rad = 270 deg
///     x       __
///   0-->     /  \
///  y|       |  x--> v2
///   v        \ |v1   
///              v
pub fn directed_angle<T>(v1: Vector2D<T>, v2: Vector2D<T>) -> f32 {
    let a = (v2.y).atan2(v2.x) - (v1.y).atan2(v1.x);
    return if a < 0.0 { a + 2.0 * PI } else { a };
}

pub fn deg(rad: f32) -> f32 {
    return rad / PI * 180.0;
}

fn get_vertex_type<T: Copy>(prev: Vector2D<T>, current: Vector2D<T>, next: Vector2D<T>) -> VertexType {
    // assuming clockwise path winding order
    let interrior_angle = directed_angle(prev - current, next - current);

    if current.y > prev.y && current.y >= next.y {
        if interrior_angle <= PI {
            return VertexType::Merge;
        } else {
            return VertexType::End;
        }
    }

    if current.y < prev.y && current.y <= next.y {
        if interrior_angle <= PI {
            return VertexType::Split;
        } else {
            return VertexType::Start;
        }
    }

    return if prev.y < next.y { VertexType::Right } else { VertexType::Left };
}

pub fn find(slice: &[EdgeId], item: EdgeId) -> Option<usize> {
    for i in 0 .. slice.len() {
        if slice[i] == item { return Some(i); }
    }
    return None;
}
pub fn sort_x<T: Copy>(slice: &mut[EdgeId], kernel: &ConnectivityKernel, path: &[Vector2D<T>]) {
    slice.sort_by(|a, b| {
        path[kernel[*a].vertex.as_index()].y.partial_cmp(&path[kernel[*b].vertex.as_index()].y).unwrap().reverse()
    });    
}

pub fn sweep_status_push<T:Copy>(
    kernel: &ConnectivityKernel,
    path: &[Vector2D<T>],
    sweep: &mut Vec<EdgeId>,
    e: &EdgeId
) {
    sweep.push(*e);
    sort_x(&mut sweep[..], kernel, path);
}

pub fn split_face(
    kernel: &mut ConnectivityKernel,
    mut a: EdgeId,
    mut b: EdgeId,
    new_faces: &mut Vec<FaceId>
) {
    let first_a = a;
    let first_b = b;
    let mut ok = false;
    loop {
        loop {
            if kernel[a].face == kernel[b].face  {
                ok = true;
                break;
            }
            a = kernel.next_edge_around_vertex(a);
            if a == first_a { break; }
        }
        if ok { break; }
        b = kernel.next_edge_around_vertex(b);
        debug_assert!(b != first_b);
    }

    if let Some(new_face) = kernel.split_face(a, b) {
        new_faces.push(new_face);
    }
}

pub fn y_monotone_decomposition<T: Copy+Debug>(
    kernel: &mut ConnectivityKernel,
    face_id: FaceId,
    path: &[Vector2D<T>],
    new_faces: &mut Vec<FaceId>
) {
    let mut sorted_edges: Vec<EdgeId> = FromIterator::from_iter(kernel.walk_edges_around_face(face_id));

    // also add holes in the sorted edge list
    for inner in kernel.face(face_id).inner_edges.iter() {
        for e in kernel.walk_edges(*inner) {
            sorted_edges.push(e);
        }
    }

    // sort indices by increasing y coordinate of the corresponding vertex
    sorted_edges.sort_by(|a, b| {
        if path[kernel[*a].vertex.as_index()].y > path[kernel[*b].vertex.as_index()].y {
            return Ordering::Greater;
        }
        if path[kernel[*a].vertex.as_index()].y < path[kernel[*b].vertex.as_index()].y {
            return Ordering::Less;
        }
        if path[kernel[*a].vertex.as_index()].x < path[kernel[*b].vertex.as_index()].x {
            return Ordering::Greater;
        }
        if path[kernel[*a].vertex.as_index()].x > path[kernel[*b].vertex.as_index()].x {
            return Ordering::Less;
        }
        return Ordering::Equal;
    });

    // list of edges that intercept the sweep line, sorted by increasing x coordinate
    let mut sweep_status: Vec<EdgeId> = vec![];
    let mut helper: HashMap<usize, (EdgeId, VertexType)> = HashMap::new();

    for e in sorted_edges.iter() {
        let edge = kernel[*e];
        let current_vertex = path[edge.vertex.as_index()];
        let previous_vertex = path[kernel[edge.prev].vertex.as_index()];
        let next_vertex = path[kernel[edge.next].vertex.as_index()];
        let vertex_type = get_vertex_type(previous_vertex, current_vertex, next_vertex);

        match vertex_type {
            VertexType::Start => {
                sweep_status_push(kernel, path, &mut sweep_status, e);
                helper.insert(e.as_index(), (*e, VertexType::Start));
            }
            VertexType::End => {
                if let Some(&(h, VertexType::Merge)) = helper.get(&edge.prev.as_index()) {
                    split_face(kernel, edge.prev, h, new_faces);
                } 
                sweep_status.retain(|item|{ *item != edge.prev });
            }
            VertexType::Split => {
                for i in 0 .. sweep_status.len() {
                    if path[kernel[sweep_status[i]].vertex.as_index()].x >= current_vertex.x {
                        if let Some(&(helper_edge,_)) = helper.get(&sweep_status[i].as_index()) {
                            split_face(kernel, *e, helper_edge, new_faces);
                        }
                        helper.insert(sweep_status[i].as_index(), (*e, VertexType::Split));
                        break;
                    }
                }
                sweep_status_push(kernel, path, &mut sweep_status, e);
                helper.insert(e.as_index(), (*e, VertexType::Split));
            }
            VertexType::Merge => {
                if let Some((h, VertexType::Merge)) = helper.remove(&edge.prev.as_index()) {
                    split_face(kernel, *e, h, new_faces);
                }
                for i in 0 .. sweep_status.len() {
                    if path[kernel[sweep_status[i]].vertex.as_index()].x > current_vertex.x {
                        if let Some((prev_helper, VertexType::Merge)) = helper.insert(
                            sweep_status[i].as_index(),
                            (*e, VertexType::Merge)
                        ) {
                            split_face(kernel, prev_helper, *e, new_faces);
                        }
                        break;
                    }
                }
            }
            VertexType::Left => {
                for i in 0 .. sweep_status.len() {
                    if path[kernel[sweep_status[i]].vertex.as_index()].x > current_vertex.x {
                        if let Some((prev_helper, VertexType::Merge)) = helper.insert(sweep_status[i].as_index(), (*e, VertexType::Right)) {
                            split_face(kernel, prev_helper, *e, new_faces);
                        }
                        break;
                    }
                }
            }
            VertexType::Right => {
                if let Some((h, VertexType::Merge)) = helper.remove(&edge.prev.as_index()) {
                    split_face(kernel, *e, h, new_faces);
                }
                sweep_status.retain(|item|{ *item != edge.prev });
                sweep_status_push(kernel, path, &mut sweep_status, e);
                helper.insert(e.as_index(), (*e, VertexType::Left));
            }
        }
    }
}

pub fn is_y_monotone<T:Copy+Debug>(kernel: &ConnectivityKernel, path: &[Vector2D<T>], face: FaceId) -> bool {
    for e in kernel.walk_edges_around_face(face) {
        let edge = kernel[e];
        let current_vertex = path[edge.vertex.as_index()];
        let previous_vertex = path[kernel[edge.prev].vertex.as_index()];
        let next_vertex = path[kernel[edge.next].vertex.as_index()];
        match get_vertex_type(previous_vertex, current_vertex, next_vertex) {
            VertexType::Split | VertexType::Merge => {
                //println!("not y monotone because of vertices {} {} {} edge {} {} {}",
                //    kernel[edge.prev].vertex.as_index(), edge.vertex.as_index(), kernel[edge.next].vertex.as_index(), 
                //    edge.prev.as_index(), e.as_index(), edge.next.as_index());
                return false;
            }
            _ => {}
        }
    }
    return true;
}

// TODO[nical] there's probably a generic Writer thingy in std
pub trait TriangleStream {
    fn write(&mut self, a: u16, b: u16, c: u16);
    fn count(&self) -> usize;
}

pub struct SliceTriangleStream<'l> {
    indices: &'l mut[u16],
    offset: usize,
}

impl<'l> TriangleStream for SliceTriangleStream<'l> {
    fn write(&mut self, a: u16, b: u16, c: u16) {
        debug_assert!(a != b);
        debug_assert!(b != c);
        debug_assert!(c != a);
        self.indices[self.offset] = a;
        self.indices[self.offset+1] = b;
        self.indices[self.offset+2] = c;
        self.offset += 3;
    }

    fn count(&self) -> usize { self.offset as usize / 3 }
}

impl<'l> SliceTriangleStream<'l> {
    pub fn new(buffer: &'l mut[u16]) -> SliceTriangleStream {
        SliceTriangleStream {
            indices: buffer,
            offset: 0,
        }
    }
}

// Returns the number of indices added
pub fn y_monotone_triangulation<T: Copy+Debug, Triangles: TriangleStream>(
    kernel: &ConnectivityKernel,
    face: FaceId,
    path: &[Vector2D<T>],
    triangles: &mut Triangles,
) {
    let first_edge = kernel[face].first_edge;
    let mut up = DirectedEdgeCirculator::new(kernel, first_edge, Direction::Forward);
    let mut down = up.clone();
    loop {
        down = down.next();
        if path[up.vertex_id().as_index()].y != path[down.vertex_id().as_index()].y {
            break;
        }
    }

    if path[up.vertex_id().as_index()].y < path[down.vertex_id().as_index()].y {
        up.set_direction(Direction::Backward);
    } else {
        down.set_direction(Direction::Backward);
    }

    // find the bottom-most vertex (with the highest y value)
    let mut big_y = path[down.vertex_id().as_index()].y;
    loop {
        debug_assert_eq!(down.face_id(), face);
        down = down.next();
        let new_y = path[down.vertex_id().as_index()].y;
        if new_y < big_y {
            down = down.prev();
            break;
        }
        big_y = new_y;
    }

    // find the top-most vertex (with the smallest y value)
    let mut small_y = path[up.vertex_id().as_index()].y;
    loop {
        debug_assert_eq!(up.face_id(), face);
        up = up.next();
        let new_y = path[up.vertex_id().as_index()].y;
        if new_y > small_y {
            up = up.prev();
            break;
        }
        small_y = new_y;
    }

    // vertices already visited, waiting to be connected
    let mut vertex_stack: Vec<DirectedEdgeCirculator> = Vec::new();
    // now that we have the top-most vertex, we will circulate simulataneously
    // from the left and right chains until we reach the bottom-most vertex

    // main chain
    let mut m = up.clone();

    // opposite chain
    let mut o = up.clone();
    m.set_direction(Direction::Forward);
    o.set_direction(Direction::Backward);

    m = m.next();
    o = o.next();

    if path[m.vertex_id().as_index()].y > path[o.vertex_id().as_index()].y {
        swap(&mut m, &mut o);
    }

    m = m.prev();
    // previous
    let mut p = m;

    let initial_triangle_count = triangles.count();
    let mut i: i32 = 0;
    loop {
        // walk edges from top to bottom, alternating between the left and 
        // right chains. The chain we are currently iterating over is the
        // main chain (m) and the other one the opposite chain (o).
        // p is the previous iteration, regardless of whcih chain it is on.
        if path[m.vertex_id().as_index()].y > path[o.vertex_id().as_index()].y || m == down {
            swap(&mut m, &mut o);
        }

        if i < 2 {
            vertex_stack.push(m);
        } else {
            if vertex_stack.len() > 0 && m.direction() != vertex_stack[vertex_stack.len()-1].direction() {
                for i in 0..vertex_stack.len() - 1 {
                    let id_1 = vertex_stack[i].vertex_id();
                    let id_2 = vertex_stack[i+1].vertex_id();
                    let id_opp = m.vertex_id();

                    triangles.write(
                        id_opp.as_index() as u16,
                        id_1.as_index() as u16,
                        id_2.as_index() as u16
                    );
                }

                vertex_stack.clear();

                vertex_stack.push(p);
                vertex_stack.push(m);

            } else {

                let mut last_popped = vertex_stack.pop();

                loop {
                    if vertex_stack.len() < 1 {
                        break;
                    }
                    let mut id_1 = vertex_stack[vertex_stack.len()-1].vertex_id();
                    let id_2 = last_popped.unwrap().vertex_id();
                    let mut id_3 = m.vertex_id();

                    if m.direction() == Direction::Backward {
                        swap(&mut id_1, &mut id_3);
                    }

                    let v1 = path[id_1.as_index()];
                    let v2 = path[id_2.as_index()];
                    let v3 = path[id_3.as_index()];
                    if directed_angle(v1 - v2, v3 - v2) > PI {
                        triangles.write(
                            id_1.as_index() as u16,
                            id_2.as_index() as u16,
                            id_3.as_index() as u16
                        );

                        last_popped = vertex_stack.pop();

                    } else {
                        break;
                    }
                } // loop 2

                if let Some(item) = last_popped {
                    vertex_stack.push(item);
                }
                vertex_stack.push(m);

            }
        }

        if m == down {
            if o == down {
                break;
            }
        }

        i += 1;
        p = m;
        m = m.next();
        debug_assert!(path[m.vertex_id().as_index()].y >= path[p.vertex_id().as_index()].y);
    }
    let num_triangles = triangles.count() - initial_triangle_count;
    debug_assert_eq!(num_triangles, kernel.count_edges_around_face(face) as usize - 2);
}

pub fn triangulate_faces<T:Copy+Debug>(
    kernel: &mut ConnectivityKernel,
    faces: &[FaceId],
    vertices: &[Vector2D<T>],
    indices: &mut[u16]
) -> usize {
    let mut new_faces: Vec<FaceId> = vec![];
    for &f in faces.iter() {
        new_faces.push(f);
    }

    for f in faces.iter() {
        y_monotone_decomposition(kernel, *f, vertices, &mut new_faces);
    }

    let mut triangles = SliceTriangleStream::new(&mut indices[..]);
    for &f in new_faces.iter() {
        debug_assert!(is_y_monotone(kernel, vertices, f));
        y_monotone_triangulation(
            kernel, f,
            vertices,
            &mut triangles
        );
    }

    return triangles.count() * 3;
}

#[test]
fn test_triangulate() {
    let paths : &[&[world::Vec2]] = &[
        &[
            world::vec2(-10.0, 5.0),
            world::vec2(0.0, -5.0),
            world::vec2(10.0, 5.0),
        ],
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
        ],
        &[
            world::vec2(1.0, 2.0),
            world::vec2(1.5, 3.0),
            world::vec2(0.0, 4.0),
            world::vec2(-1.0, 1.0),
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(2.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 1.0),
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 1.0),// <
            world::vec2(2.0, 0.0),//  |
            world::vec2(2.0, 4.0),//  |
            world::vec2(1.0, 3.0),// <
            world::vec2(0.0, 4.0),
        ],
        &[
            world::vec2(0.0, 2.0),
            world::vec2(1.0, 2.0),
            world::vec2(0.0, 1.0),
            world::vec2(2.0, 0.0),
            world::vec2(3.0, 1.0),// 4
            world::vec2(4.0, 0.0),
            world::vec2(3.0, 2.0),
            world::vec2(2.0, 1.0),// 7
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 4.0)
        ],
        &[
            world::vec2(0.0, 0.0),
            world::vec2(1.0, 0.0),
            world::vec2(2.0, 0.0),
            world::vec2(3.0, 0.0),
            world::vec2(3.0, 1.0),
            world::vec2(3.0, 2.0),
            world::vec2(3.0, 3.0),
            world::vec2(2.0, 3.0),
            world::vec2(1.0, 3.0),
            world::vec2(0.0, 3.0),
            world::vec2(0.0, 2.0),
            world::vec2(0.0, 1.0),
        ],
    ];

    let indices = &mut [0 as u16; 1024];
    for i in 0 .. paths.len() {
        println!("\n\n -- path {:?}", i);
        let mut kernel = ConnectivityKernel::from_loop(paths[i].len() as u16);
        let main_face = kernel.first_face();
        let n_indices = triangulate_faces(&mut kernel, &[main_face], &paths[i][..], indices);
        for n in 0 .. n_indices/3 {
            println!(" ===> {} {} {}", indices[n*3], indices[n*3+1], indices[n*3+2] );
        }
    }
}

#[test]
fn test_triangulate_holes() {
    let paths : &[(&[world::Vec2], &[u16])] = &[
        (
            &[
                // outer
                world::vec2(-11.0, 5.0),
                world::vec2(0.0, -5.0),
                world::vec2(10.0, 5.0),
                // hole
                world::vec2(4.0, 2.0),
                world::vec2(0.0, -2.0),
                world::vec2(-5.0, 2.0),
            ],
            &[ 3, 3 ]
        ),
        (
            &[
                // outer
                world::vec2(-10.0, -10.0),
                world::vec2( 10.0, -10.0),
                world::vec2( 10.0,  10.0),
                world::vec2(-10.0,  10.0),
                // hole
                world::vec2(4.0, 2.0),
                world::vec2(0.0, -2.0),
                world::vec2(-4.0, 2.0),
            ],
            &[ 4, 3 ]
        ),
        (
            &[
                // outer
                world::vec2(-10.0, -10.0),
                world::vec2( 10.0, -10.0),
                world::vec2( 10.0,  10.0),
                world::vec2(-10.0,  10.0),
                // hole 1
                world::vec2(-8.0, 8.0),
                world::vec2(4.0, 8.0),
                world::vec2(-4.0, -8.0),
                world::vec2(-8.0, -8.0),
                // hole 2
                world::vec2(-2.0, -8.0),
                world::vec2(6.0, 7.0),
                world::vec2(8.0, -8.0),
            ],
            &[ 4, 4, 3 ]
        ),
    ];

    let indices = &mut [0 as u16; 1024];
    for i in 0 .. paths.len() {
        println!("\n\n -- path {:?}", i);
        let &(vertices, separators) = &paths[i];

        let mut kernel = ConnectivityKernel::from_loop(separators[0]);
        let main_face = kernel.first_face();
        for i in 1 .. separators.len() {
            kernel.add_hole(main_face, separators[i]);
        }

        let n_indices = triangulate_faces(&mut kernel, &[main_face], vertices, indices);
        for n in 0 .. n_indices/3 {
            println!(" ===> {} {} {}", indices[n*3], indices[n*3+1], indices[n*3+2] );
        }
    }
}