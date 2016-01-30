use half_edge::kernel::{ VertexId, VertexIdRange, vertex_id };
use half_edge::vectors::{ Vec2, vec2_sub, directed_angle, Position2D };

use vodk_id::id_vector::IdSlice;
use vodk_id::{ Id, IdRange };
use tesselation::path::WindingOrder;
use std::f32::consts::PI;

pub fn vertex_id_range(from: u16, to: u16) -> VertexIdRange {
    IdRange {
        first: Id::new(from),
        count: to - from,
    }
}

#[derive(Debug)]
pub struct Point_;
pub type PointId = Id<Point_, u16>;
pub fn point_id(idx: u16) -> PointId { PointId::new(idx) }

pub use half_edge::iterators::{Direction };

//#[derive(Copy, Clone, Debug)]
//pub enum Direction {
//    Forward,
//    Backward,
//}
//
//impl Direction {
//    pub fn reverse(self) -> Direction {
//        match self {
//            Direction::Forward => { Direction::Backward }
//            Direction::Backward => { Direction::Forward }
//        }
//    }
//}

pub trait AbstractPolygon {
    type PointId: Copy + Eq + ::std::fmt::Debug;

    fn first_point(&self) -> Self::PointId;

    fn vertex(&self, point: Self::PointId) -> VertexId;

    fn next(&self, point: Self::PointId) -> Self::PointId;

    fn previous(&self, point: Self::PointId) -> Self::PointId;

    fn advance(&self, point: Self::PointId, dir: Direction) -> Self::PointId {
        return match dir {
            Direction::Forward => { self.next(point) }
            Direction::Backward => { self.previous(point) }
        };
    }

    fn next_vertex(&self, point: Self::PointId) -> VertexId {
        self.vertex(self.next(point))
    }

    fn previous_vertex(&self, point: Self::PointId) -> VertexId {
        self.vertex(self.previous(point))
    }

    // number of vertices on the loop containing point
    fn num_vertices_on_loop(&self, point: Self::PointId) -> usize;

    // number of vertices total
    fn num_vertices(&self) -> usize;

    fn get_sub_polygon<'l>(&'l self, id: PolygonId) -> Option<PolygonView<'l>>;
}

pub trait AbstractCirculator {
    type PointId: Copy + Eq + ::std::fmt::Debug;

    fn point(self) -> Self::PointId;

    fn vertex(self) -> VertexId;

    fn next(&mut self);

    fn previous(&mut self);
}

#[derive(Copy, Clone)]
pub struct PolygonView<'l> {
    vertices: &'l [VertexId]
}

impl<'l> PolygonView<'l> {
    pub fn iter_from(self, point: PointId) -> PolygonIterator<'l>
    {
        self.circulator_at(point).iter()
    }

    pub fn circulator_at(self, point: PointId) -> PolygonCirculator<'l> {
        PolygonCirculator {
            polygon: self,
            point: point
        }
    }

    pub fn circulator(self) -> PolygonCirculator<'l> {
        self.circulator_at(point_id(0))
    }

    pub fn point_ids(self) -> IdRange<Point_, u16> {
        IdRange {
            first: point_id(0),
            count: self.vertices.len() as u16
        }
    }
}

impl<'l> AbstractPolygon for PolygonView<'l> {
    type PointId = PointId;

    fn first_point(&self) -> PointId { point_id(0) }

    fn vertex(&self, point: PointId) -> VertexId {
        self.vertices[point.handle as usize]
    }

    fn next(&self, point: PointId) -> PointId {
        point_id((point.handle + 1) % (self.vertices.len() as u16))
    }

    fn previous(&self, point: PointId) -> PointId {
        point_id(if point.handle == 0 { self.vertices.len() as u16 - 1 }  else { point.handle - 1 })
    }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn num_vertices_on_loop(&self, _point: PointId) -> usize { self.num_vertices() }

    fn get_sub_polygon<'m>(&'m self, _: PolygonId) -> Option<PolygonView<'m>> { None }
}

#[derive(Clone)]
pub struct Polygon {
    pub vertices: Vec<VertexId>,
}

impl Polygon {
    pub fn new() -> Polygon { Polygon { vertices: Vec::new() } }

    pub fn from_vertices<It: Iterator<Item=VertexId>>(it: It) -> Polygon {
        let (lower_bound, _) = it.size_hint();
        let mut v = Vec::with_capacity(lower_bound);
        v.extend(it);
        Polygon {
            vertices: v
        }
    }

    pub fn is_empty(&self) -> bool { self.vertices.is_empty() }

    pub fn into_complex_polygon(self) -> ComplexPolygon {
        ComplexPolygon {
            main: self,
            holes: Vec::new()
        }
    }

    /// Add vertex to the end
    pub fn push_vertex(&mut self, v: VertexId) -> PointId {
        self.vertices.push(v);
        return Id::new(self.vertices.len() as u16 - 1);
    }

    /// Add vertex to the end
    pub fn remove_vertex(&mut self, v: PointId) -> VertexId {
        self.vertices.remove(v.handle as usize)
    }

    /// Retains only the elements specified by the predicate (seimilar to std::vec::Vec::retain).
    pub fn retain_vertices<F>(&mut self, f: F) where F: FnMut(&VertexId) -> bool {
        self.vertices.retain(f)
    }

    /// Insert a vertex for a given point_id shifting all elements after that position to the right.
    pub fn insert_vertex(&mut self, point: PointId, new_vertex: VertexId) {
        self.vertices.insert(point.handle as usize, new_vertex);
    }

    pub fn view<'l>(&'l self) -> PolygonView<'l> {
        PolygonView { vertices: &self.vertices[..] }
    }

    pub fn iter_from<'l>(&'l self, point: PointId) -> PolygonIterator<'l> {
        self.circulator_at(point).iter()
    }

    pub fn circulator_at<'l>(&'l self, point: PointId) -> PolygonCirculator<'l> {
        PolygonCirculator {
            polygon: self.view(),
            point: point
        }
    }

    pub fn circulator<'l>(&'l self) -> PolygonCirculator<'l> {
        self.circulator_at(point_id(0))
    }
}

impl AbstractPolygon for Polygon {
    type PointId = PointId;

    fn first_point(&self) -> PointId { point_id(0) }

    fn vertex(&self, point: PointId) -> VertexId { self.vertices[point.handle as usize] }

    fn next(&self, point: PointId) -> PointId { self.view().next(point) }

    fn previous(&self, point: PointId) -> PointId { self.view().previous(point) }

    fn num_vertices(&self) -> usize { self.vertices.len() }

    fn num_vertices_on_loop(&self, _point: PointId) -> usize { self.num_vertices() }

    fn get_sub_polygon<'l>(&'l self, _: PolygonId) -> Option<PolygonView<'l>> { None }
}

pub struct ComplexPolygon {
    pub main: Polygon,
    pub holes: Vec<Polygon>
}

impl ComplexPolygon {
    pub fn new() -> ComplexPolygon {
        ComplexPolygon {
            main: Polygon::new(),
            holes: Vec::new(),
        }
    }

    pub fn add_hole(&mut self, hole: Polygon) {
        self.holes.push(hole);
    }

    pub fn polygon(&self, id: PolygonId) -> &Polygon {
        if id.handle == 0 {
            return &self.main;
        }
        return &self.holes[id.handle as usize - 1];
    }

    pub fn circulator_at<'l>(&'l self, point: ComplexPointId) -> ComplexPolygonCirculator<'l> {
        ComplexPolygonCirculator {
            circulator: PolygonCirculator {
                polygon: self.polygon(point.polygon_id).view(),
                point: point.point,
            },
            polygon_id: point.polygon_id,
        }
    }

    pub fn circulator<'l>(&'l self, id: PolygonId) -> ComplexPolygonCirculator<'l> {
        self.circulator_at(ComplexPointId { point: point_id(0), polygon_id: id })
    }

    pub fn point_ids(&self, p: PolygonId) -> ComplexPointIdRange {
        ComplexPointIdRange {
            range: IdRange {
                first: point_id(0),
                count: self.polygon(p).num_vertices() as u16
            },
            polygon_id: p,
        }
    }

    pub fn polygon_ids(&self) -> IdRange<Polygon_, u16> {
        IdRange {
            first: polygon_id(0),
            count: self.holes.len() as u16 + 1,
        }
    }
}

pub struct ComplexPointIdRange {
    range: IdRange<Point_, u16>,
    polygon_id: PolygonId,
}

impl Iterator for ComplexPointIdRange {
    type Item = ComplexPointId;
    fn next(&mut self) -> Option<ComplexPointId> {
        return if let Some(next) = self.range.next() {
            Some(ComplexPointId {
                point: next,
                polygon_id: self.polygon_id
            })
        } else {
            None
        };
    }
}

impl AbstractPolygon for ComplexPolygon {
    type PointId = ComplexPointId;

    fn first_point(&self) -> ComplexPointId {
        ComplexPointId { point: self.main.first_point(), polygon_id: polygon_id(0) }
    }

    fn vertex(&self, id: ComplexPointId) -> VertexId {
        self.polygon(id.polygon_id).vertex(id.point)
    }

    fn next(&self, id: ComplexPointId) -> ComplexPointId {
        ComplexPointId {
            point: self.polygon(id.polygon_id).next(id.point),
            polygon_id: id.polygon_id
        }
    }

    fn previous(&self, id: ComplexPointId) -> ComplexPointId {
        ComplexPointId {
            point: self.polygon(id.polygon_id).previous(id.point),
            polygon_id: id.polygon_id
        }
    }

    fn num_vertices(&self) -> usize {
        let mut result = self.main.num_vertices();
        for hole in &self.holes {
            result += hole.num_vertices();
        }
        return result;
    }

    fn num_vertices_on_loop(&self, point: ComplexPointId) -> usize {
        self.polygon(point.polygon_id).num_vertices()
    }

    fn get_sub_polygon<'l>(&'l self, id: PolygonId) -> Option<PolygonView<'l>> {
        if id.handle == 0 {
            return Some(self.main.view());
        }

        if id.handle <= self.holes.len() as u16 {
            return Some(self.holes[id.handle as usize - 1].view());
        }

        return None;
    }
}

#[derive(Copy, Clone)]
pub struct PolygonCirculator<'l> {
    polygon: PolygonView<'l>,
    point: PointId,
}

impl<'l> PolygonCirculator<'l> {
    pub fn next_vertex(self) -> VertexId {
        self.polygon.next_vertex(self.point)
    }

    pub fn previous_vertex(self) -> VertexId {
        self.polygon.previous_vertex(self.point)
    }

    pub fn advance(&mut self, dir: Direction) {
        self.point = self.polygon.advance(self.point, dir);
    }

    pub fn iter(self) -> PolygonIterator<'l> {
        PolygonIterator {
            polygon: self.polygon,
            first: self.point.handle,
            count: 0
        }
    }
}

impl<'l> AbstractCirculator for PolygonCirculator<'l> {
    type PointId = PointId;

    fn point(self) -> PointId { self.point }

    fn vertex(self) -> VertexId { self.polygon.vertex(self.point) }

    fn next(&mut self) { self.point = self.polygon.next(self.point); }

    fn previous(&mut self) { self.point = self.polygon.previous(self.point); }
}


#[derive(Copy, Clone)]
pub struct ComplexPolygonCirculator<'l> {
    circulator: PolygonCirculator<'l>,
    polygon_id: PolygonId,
}

impl<'l> ComplexPolygonCirculator<'l> {
    pub fn point(self) -> ComplexPointId {
        ComplexPointId {
            point: self.circulator.point,
            polygon_id: self.polygon_id
        }
    }

    pub fn vertex(self) -> VertexId { self.circulator.vertex() }

    pub fn next_vertex(self) -> VertexId { self.circulator.next_vertex() }

    pub fn previous_vertex(self) -> VertexId { self.circulator.previous_vertex() }

    pub fn advance(&mut self, dir: Direction) { self.circulator.advance(dir); }

    pub fn next(&mut self) { self.circulator.next(); }

    pub fn previous(&mut self) { self.circulator.previous(); }

    pub fn iter(self) -> PolygonIterator<'l> {
        PolygonIterator {
            polygon: self.circulator.polygon,
            first: self.circulator.point.handle,
            count: 0
        }
    }
}

#[derive(Copy, Clone)]
pub struct PolygonIterator<'l> {
    polygon: PolygonView<'l>,
    first: u16,
    count: u16,
}

impl<'l> Iterator for PolygonIterator<'l> {
    type Item = VertexId;

    fn next(&mut self) -> Option<VertexId> {
        let num_vertices = self.polygon.num_vertices();
        if self.count as usize >= num_vertices {
            return None;
        }

        let idx = (self.count + self.first) as usize % num_vertices;
        self.count += 1;

        return Some(self.polygon.vertices[idx]);
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let num_vertices = self.polygon.num_vertices();
        (num_vertices, Some(num_vertices))
    }
}

#[derive(Copy, Clone)]
pub struct ComplexPolygonIterator<'l> {
    iter: PolygonIterator<'l>,
    polygon_id: PolygonId,
}

impl<'l> Iterator for ComplexPolygonIterator<'l> {
    type Item = VertexId;

    fn next(&mut self) -> Option<VertexId> { self.iter.next() }

    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}


#[derive(Debug)]
pub struct Polygon_;
pub type PolygonId = Id<Polygon_, u16>;
pub fn polygon_id(idx: u16) -> PolygonId { PolygonId::new(idx) }

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ComplexPointId {
    pub point: PointId,
    pub polygon_id: PolygonId,
}

pub fn compute_winding_order<'l, Pos: Position2D>(
    poly: PolygonView<'l>,
    vertices: IdSlice<VertexId, Pos>
) -> Option<WindingOrder> {
    if poly.num_vertices() < 3 {
        return None;
    }

    let mut angle = 0.0;
    for it in poly.point_ids() {
        let a = vertices[poly.previous_vertex(it)].position();
        let b = vertices[poly.vertex(it)].position();
        let c = vertices[poly.next_vertex(it)].position();

        angle += directed_angle(vec2_sub(a, b), vec2_sub(c, b));
    }

    return if angle > ((poly.num_vertices()-1) as f32) * PI {
        Some(WindingOrder::Clockwise)
    } else {
        Some(WindingOrder::CounterClockwise)
    };
}

#[test]
fn test_simple_polygon() {
  let poly = Polygon {
    vertices: vec![
        vertex_id(0),
        vertex_id(1),
        vertex_id(2),
        vertex_id(3),
        vertex_id(4),
    ]
  };

  for v in poly.circulator_at(point_id(1)).iter() {
    println!("{}", v.handle);
  }

  let _ = poly.into_complex_polygon();
}

#[test]
fn test_winding_order()
{
    let positions: &[Vec2] = &[
        [0.0, 0.0],
        [0.0,-1.0],
        [0.0,-2.0],
        [1.0,-2.0],
        [2.0,-2.0],
        [2.0,-1.0],
        [2.0, 0.0],
        [1.0, 0.0],
    ];
    let vertices = IdSlice::new(positions);
    let poly = Polygon::from_vertices(vertex_id_range(0, 8));
    assert_eq!(compute_winding_order(poly.view(), vertices), Some(WindingOrder::Clockwise));

    let positions: &[Vec2] = &[
        [1.0, 0.0],
        [2.0, 0.0],
        [2.0,-1.0],
        [2.0,-2.0],
        [1.0,-2.0],
        [0.0,-2.0],
        [0.0,-1.0],
        [0.0, 0.0],
    ];
    let vertices = IdSlice::new(positions);
    let poly = Polygon::from_vertices(vertex_id_range(0, 8));
    assert_eq!(compute_winding_order(poly.view(), vertices), Some(WindingOrder::CounterClockwise));

}