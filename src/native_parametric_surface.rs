//! Typed evaluators for the non-planar surfaces observed in saved graphs.

use crate::native_parameters::ObjectGraph;
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

pub type Point = [f64; 3];
fn p(v: &Value, n: &str) -> Result<Point> {
    let a = v
        .as_array()
        .with_context(|| format!("{n} must be an array"))?;
    ensure!(a.len() == 3, "{n} must have three coordinates");
    let r = [
        a[0].as_f64().context("coordinate")?,
        a[1].as_f64().context("coordinate")?,
        a[2].as_f64().context("coordinate")?,
    ];
    ensure!(
        r.iter().all(|x| x.is_finite()),
        "{n} has nonfinite coordinate"
    );
    Ok(r)
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: Point, s: f64) -> Point {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn len(a: Point) -> f64 {
    dot(a, a).sqrt()
}
fn range(v: &Value, n: &str) -> Result<[f64; 2]> {
    let a = v
        .as_array()
        .with_context(|| format!("{n} must be an array"))?;
    ensure!(a.len() == 2, "{n} must have two values");
    let r = [
        a[0].as_f64().context("range")?,
        a[1].as_f64().context("range")?,
    ];
    ensure!(
        r.iter().all(|x| x.is_finite()) && r[1] > r[0],
        "invalid {n}"
    );
    Ok(r)
}
fn corner(v: &Value) -> Result<[f64; 2]> {
    let a = v.as_array().context("envelope corner")?;
    ensure!(a.len() == 2, "envelope corner must have two coordinates");
    let r = [
        a[0].as_f64().context("envelope coordinate")?,
        a[1].as_f64().context("envelope coordinate")?,
    ];
    ensure!(
        r.iter().all(|x| x.is_finite()),
        "nonfinite envelope coordinate"
    );
    Ok(r)
}

fn target(g: &ObjectGraph, source: usize, v: &Value) -> Result<usize> {
    crate::native_saved_mesh::pointer(g, source, v)?.context("missing surface pointer target")
}

fn basis(f: &Value) -> Result<(Point, Point, Point)> {
    let x = p(&f["m_xVec"], "surface x basis")?;
    let y = p(&f["m_yVec"], "surface y basis")?;
    let z = p(&f["m_zVec"], "surface z basis")?;
    ensure!(
        (len(x) - 1.).abs() <= 1e-6 && (len(y) - 1.).abs() <= 1e-6 && (len(z) - 1.).abs() <= 1e-6,
        "surface basis is not unit"
    );
    ensure!(
        dot(x, y).abs() <= 1e-6 && dot(x, z).abs() <= 1e-6 && dot(y, z).abs() <= 1e-6,
        "surface basis is not orthogonal"
    );
    Ok((x, y, z))
}

pub fn surface_of_revolution_point(g: &ObjectGraph, si: usize, u: f64, v: f64) -> Result<Point> {
    let surface = ParametricSurface::read(g, si)?;
    ensure!(
        matches!(surface, ParametricSurface::SurfRev { .. }),
        "not SurfRev"
    );
    surface.evaluate(u, v)
}

pub fn ruled_surface_point(g: &ObjectGraph, si: usize, u: f64, v: f64) -> Result<Point> {
    let surface = ParametricSurface::read(g, si)?;
    ensure!(
        matches!(surface, ParametricSurface::RuledSurf { .. }),
        "not RuledSurf"
    );
    surface.evaluate(u, v)
}

#[derive(Debug, Clone)]
pub enum Curve {
    Line {
        origin: Point,
        direction: Point,
        range: [f64; 2],
    },
    Arc {
        center: Point,
        x: Point,
        y: Point,
        radius: f64,
        range: [f64; 2],
    },
    /// Piecewise cubic Hermite: (parameter, point, tangent) nodes, the
    /// tangent being the derivative in that parameter.
    Hermite {
        nodes: Vec<(f64, Point, Point)>,
        range: [f64; 2],
    },
    /// B-spline, rational when the weights differ from one.
    Nurbs {
        degree: usize,
        knots: Vec<f64>,
        points: Vec<Point>,
        weights: Vec<f64>,
        range: [f64; 2],
    },
}

/// The span of `t` in increasing breakpoints: the largest i with
/// `params[i] <= t`, kept below the last breakpoint.
fn span(params: &[f64], t: f64) -> usize {
    params
        .partition_point(|x| *x <= t)
        .saturating_sub(1)
        .min(params.len() - 2)
}

/// Cubic Hermite basis (h00, h10, h01, h11) or its `order`-th derivative
/// in the local parameter s.
fn hermite_basis(s: f64, order: usize) -> [f64; 4] {
    match order {
        0 => [
            2. * s * s * s - 3. * s * s + 1.,
            s * s * s - 2. * s * s + s,
            -2. * s * s * s + 3. * s * s,
            s * s * s - s * s,
        ],
        1 => [
            6. * s * s - 6. * s,
            3. * s * s - 4. * s + 1.,
            -6. * s * s + 6. * s,
            3. * s * s - 2. * s,
        ],
        _ => [12. * s - 6., 6. * s - 4., -12. * s + 6., 6. * s - 2.],
    }
}

/// Nonzero B-spline basis values of `degree` at t in knot span `span`:
/// the functions span-degree..=span.
fn bspline_basis(knots: &[f64], span: usize, t: f64, degree: usize) -> Vec<f64> {
    let mut n = vec![0.; degree + 1];
    let mut left = vec![0.; degree + 1];
    let mut right = vec![0.; degree + 1];
    n[0] = 1.;
    for j in 1..=degree {
        left[j] = t - knots[span + 1 - j];
        right[j] = knots[span + j] - t;
        let mut saved = 0.;
        for r in 0..j {
            let d = right[r + 1] + left[j - r];
            let temp = if d == 0. { 0. } else { n[r] / d };
            n[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        n[j] = saved;
    }
    n
}

impl Curve {
    pub fn line(origin: Point, direction: Point, range: [f64; 2]) -> Self {
        Self::Line {
            origin,
            direction,
            range,
        }
    }
    fn read(g: &ObjectGraph, i: usize) -> Result<Self> {
        let o = g.objects.get(i).context("profile object")?;
        match o.class_name.as_str() {
            "GLine" => {
                let direction = p(&o.fields["m_dirVec"], "line direction")?;
                ensure!(len(direction) > 1e-12, "singular line direction");
                Ok(Self::Line {
                    origin: p(&o.fields["m_origin"], "line origin")?,
                    direction,
                    range: range(&o.fields["m_endParams"], "line range")?,
                })
            }
            "GArc" => {
                let x = p(&o.fields["m_xVec"], "arc x basis")?;
                let y = p(&o.fields["m_yVec"], "arc y basis")?;
                let radius = o.fields["m_radius"].as_f64().context("arc radius")?;
                ensure!(
                    radius.is_finite()
                        && radius > 0.
                        && (len(x) - 1.).abs() <= 1e-6
                        && (len(y) - 1.).abs() <= 1e-6
                        && dot(x, y).abs() <= 1e-6,
                    "invalid arc basis"
                );
                Ok(Self::Arc {
                    center: p(&o.fields["m_center"], "arc center")?,
                    x,
                    y,
                    radius,
                    range: range(&o.fields["m_endParams"], "arc range")?,
                })
            }
            "GHermiteSpline" => {
                ensure!(
                    o.fields["m_Periodic"] == false,
                    "periodic Hermite spline unsupported"
                );
                let nodes = o.fields["m_NodeArray"]
                    .as_array()
                    .context("Hermite nodes")?
                    .iter()
                    .map(|n| {
                        let t = n["m_iParametr"].as_f64().context("Hermite parameter")?;
                        ensure!(t.is_finite(), "nonfinite Hermite parameter");
                        Ok((
                            t,
                            p(&n["m_iPoint"], "Hermite point")?,
                            p(&n["m_iTangent"], "Hermite tangent")?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    nodes.len() >= 2 && nodes.windows(2).all(|w| w[1].0 > w[0].0),
                    "Hermite parameters must increase"
                );
                let range = range(&o.fields["m_endParams"], "Hermite range")?;
                ensure!(
                    range[0] >= nodes[0].0 - 1e-9 && range[1] <= nodes[nodes.len() - 1].0 + 1e-9,
                    "Hermite range outside nodes"
                );
                Ok(Self::Hermite { nodes, range })
            }
            "GNurbSpline" => {
                let degree = o.fields["m_Degree"].as_u64().context("NURBS degree")? as usize;
                let knots = o.fields["m_Knots"]
                    .as_array()
                    .context("NURBS knots")?
                    .iter()
                    .map(|k| k.as_f64().context("NURBS knot"))
                    .collect::<Result<Vec<_>>>()?;
                let mut points = Vec::new();
                let mut weights = Vec::new();
                for c in o.fields["m_ctrlPoints"]
                    .as_array()
                    .context("NURBS control points")?
                {
                    points.push(p(&c["m_Point"], "NURBS control point")?);
                    let w = c["m_w"].as_f64().context("NURBS weight")?;
                    ensure!(w.is_finite() && w > 0., "invalid NURBS weight");
                    weights.push(w);
                }
                if o.fields["m_Rational"] == false {
                    ensure!(
                        weights.iter().all(|w| *w == 1.),
                        "nonrational NURBS with weights"
                    );
                }
                ensure!(
                    degree >= 1
                        && points.len() > degree
                        && knots.len() == points.len() + degree + 1
                        && knots.iter().all(|k| k.is_finite())
                        && knots.windows(2).all(|w| w[1] >= w[0]),
                    "invalid NURBS knots"
                );
                let range = range(&o.fields["m_endParams"], "NURBS range")?;
                ensure!(
                    range[0] >= knots[degree] - 1e-9 && range[1] <= knots[points.len()] + 1e-9,
                    "NURBS range outside knots"
                );
                Ok(Self::Nurbs {
                    degree,
                    knots,
                    points,
                    weights,
                    range,
                })
            }
            name => bail!("unsupported parametric profile curve {name}"),
        }
    }
    pub(crate) fn eval(&self, t: f64) -> Point {
        self.point_and_derivative(t).0
    }
    pub(crate) fn derivative(&self, t: f64) -> Point {
        self.point_and_derivative(t).1
    }
    fn point_and_derivative(&self, t: f64) -> (Point, Point) {
        match self {
            Self::Line {
                origin, direction, ..
            } => (add(*origin, mul(*direction, t)), *direction),
            Self::Arc {
                center,
                x,
                y,
                radius,
                ..
            } => (
                add(
                    *center,
                    mul(add(mul(*x, t.cos()), mul(*y, t.sin())), *radius),
                ),
                mul(add(mul(*x, -t.sin()), mul(*y, t.cos())), *radius),
            ),
            Self::Hermite { nodes, .. } => {
                let i = nodes
                    .partition_point(|n| n.0 <= t)
                    .saturating_sub(1)
                    .min(nodes.len() - 2);
                let (t0, p0, d0) = nodes[i];
                let (t1, p1, d1) = nodes[i + 1];
                let h = t1 - t0;
                let s = (t - t0) / h;
                let combine = |b: [f64; 4]| {
                    add(
                        add(mul(p0, b[0]), mul(d0, h * b[1])),
                        add(mul(p1, b[2]), mul(d1, h * b[3])),
                    )
                };
                (
                    combine(hermite_basis(s, 0)),
                    mul(combine(hermite_basis(s, 1)), 1. / h),
                )
            }
            Self::Nurbs {
                degree,
                knots,
                points,
                weights,
                ..
            } => {
                let p = *degree;
                let s = knots
                    .partition_point(|k| *k <= t)
                    .saturating_sub(1)
                    .clamp(p, points.len() - 1);
                let n = bspline_basis(knots, s, t, p);
                let lower = bspline_basis(knots, s, t, p - 1);
                let ratio = |a: f64, d: f64| if d == 0. { 0. } else { a / d };
                let (mut a, mut w, mut da, mut dw) = ([0.; 3], 0., [0.; 3], 0.);
                for r in 0..=p {
                    let i = s - p + r;
                    let left = if r > 0 {
                        ratio(lower[r - 1], knots[i + p] - knots[i])
                    } else {
                        0.
                    };
                    let right = if r < p {
                        ratio(lower[r], knots[i + p + 1] - knots[i + 1])
                    } else {
                        0.
                    };
                    let dn = p as f64 * (left - right);
                    a = add(a, mul(points[i], n[r] * weights[i]));
                    w += n[r] * weights[i];
                    da = add(da, mul(points[i], dn * weights[i]));
                    dw += dn * weights[i];
                }
                let c = mul(a, 1. / w);
                (c, mul(sub(da, mul(c, dw)), 1. / w))
            }
        }
    }
    /// Parameters splitting the range so that each piece sags at most
    /// `chord`: every polynomial span is cut evenly by its own step. Within
    /// a span the sag of a step h is at most B h^2 / 8, B the span's largest
    /// second derivative (finite differences of the derivative); a span that
    /// sags less than `chord` as a whole, like the near-zero span of a
    /// corner, stays whole.
    pub(crate) fn chord_params(&self, chord: f64) -> Vec<f64> {
        let r = c_range(self);
        let mut breaks = match self {
            Self::Line { .. } | Self::Arc { .. } => vec![],
            Self::Hermite { nodes, .. } => nodes.iter().map(|n| n.0).collect(),
            Self::Nurbs { knots, .. } => knots.clone(),
        };
        breaks.extend(r);
        breaks.retain(|t| *t >= r[0] && *t <= r[1]);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        let mut params = vec![r[0]];
        for w in breaks.windows(2) {
            let width = w[1] - w[0];
            let h = width * 1e-4;
            let mut bound = 0_f64;
            for k in 0..=8 {
                let t = (w[0] + width * k as f64 / 8.).clamp(w[0] + h, w[1] - h);
                let d = sub(self.derivative(t + h), self.derivative(t - h));
                bound = bound.max(len(d) / (2. * h));
            }
            let parts = if bound * width * width / 8. > chord {
                (width / (8. * chord / bound).sqrt()).ceil() as usize
            } else {
                1
            };
            params.extend((1..=parts).map(|k| w[0] + width * k as f64 / parts as f64));
        }
        params
    }
}

#[derive(Debug, Clone)]
pub enum ParametricSurface {
    SurfRev {
        center: Point,
        x: Point,
        y: Point,
        z: Point,
        envelope: [[f64; 2]; 2],
        orient: bool,
        profile: Curve,
    },
    RuledSurf {
        envelope: [[f64; 2]; 2],
        orient: bool,
        profile1: Curve,
        profile2: Curve,
    },
    /// Bicubic Hermite patches over the `us` x `vs` breakpoints. Nodes run
    /// u-fastest and hold the point, its u and v derivatives and the mixed
    /// derivative.
    HermiteSurf {
        envelope: [[f64; 2]; 2],
        orient: bool,
        us: Vec<f64>,
        vs: Vec<f64>,
        nodes: Vec<[Point; 4]>,
    },
}

/// The `order_u`, `order_v` derivative of a bicubic Hermite surface.
fn hermite_patch(
    us: &[f64],
    vs: &[f64],
    nodes: &[[Point; 4]],
    u: f64,
    v: f64,
    order_u: usize,
    order_v: usize,
) -> Point {
    let (i, j) = (span(us, u), span(vs, v));
    let (h, k) = (us[i + 1] - us[i], vs[j + 1] - vs[j]);
    let bu = hermite_basis((u - us[i]) / h, order_u);
    let bv = hermite_basis((v - vs[j]) / k, order_v);
    let mut r = [0.; 3];
    for b in 0..2 {
        for a in 0..2 {
            let [q, qu, qv, quv] = nodes[(j + b) * us.len() + i + a];
            let (pu, tu) = (bu[2 * a], h * bu[2 * a + 1]);
            let (pv, tv) = (bv[2 * b], k * bv[2 * b + 1]);
            r = add(
                r,
                add(
                    add(mul(q, pu * pv), mul(qu, tu * pv)),
                    add(mul(qv, pu * tv), mul(quv, tu * tv)),
                ),
            );
        }
    }
    mul(r, 1. / (h.powi(order_u as i32) * k.powi(order_v as i32)))
}

impl ParametricSurface {
    pub fn read(g: &ObjectGraph, si: usize) -> Result<Self> {
        let s = g.objects.get(si).context("surface index")?;
        let corners = s.fields["m_Envelope"]["m_corners"]
            .as_array()
            .context("surface envelope")?;
        ensure!(corners.len() == 2, "surface envelope corners");
        let a = corner(&corners[0])?;
        let b = corner(&corners[1])?;
        ensure!(
            b[0] > a[0] && b[1] > a[1],
            "surface envelope is not ordered"
        );
        let envelope = [a, b];
        match s.class_name.as_str() {
            "SurfRev" => {
                let profile = Curve::read(g, target(g, si, &s.fields["m_pProfileCurve"])?)?;
                let (x, y, z) = basis(&s.fields)?;
                Ok(Self::SurfRev {
                    center: p(&s.fields["m_center"], "surface center")?,
                    x,
                    y,
                    z,
                    envelope,
                    orient: s.fields["m_orientFlag"]
                        .as_bool()
                        .context("surface orientation")?,
                    profile,
                })
            }
            "RuledSurf" => Ok(Self::RuledSurf {
                envelope,
                orient: s.fields["m_orientFlag"]
                    .as_bool()
                    .context("surface orientation")?,
                profile1: Curve::read(g, target(g, si, &s.fields["m_pProfileCurve1"])?)?,
                profile2: Curve::read(g, target(g, si, &s.fields["m_pProfileCurve2"])?)?,
            }),
            "HermiteSurf" => {
                ensure!(
                    s.fields["m_periodic"] == serde_json::json!([false, false]),
                    "periodic Hermite surface unsupported"
                );
                let params = |n: &str| -> Result<Vec<f64>> {
                    let a = s.fields[n]
                        .as_array()
                        .with_context(|| format!("{n} must be an array"))?
                        .iter()
                        .map(|x| x.as_f64().context("Hermite surface parameter"))
                        .collect::<Result<Vec<_>>>()?;
                    ensure!(
                        a.len() >= 2
                            && a.iter().all(|x| x.is_finite())
                            && a.windows(2).all(|w| w[1] > w[0]),
                        "{n} must increase"
                    );
                    Ok(a)
                };
                let (us, vs) = (params("m_uParams")?, params("m_vParams")?);
                let nodes = s.fields["m_NodeArray"]
                    .as_array()
                    .context("Hermite surface nodes")?
                    .iter()
                    .map(|n| {
                        let t = n["m_iTangent"]
                            .as_array()
                            .context("Hermite surface tangents")?;
                        ensure!(t.len() == 2, "Hermite surface tangent count");
                        Ok([
                            p(&n["m_iPoint"], "Hermite surface point")?,
                            p(&t[0], "Hermite surface u tangent")?,
                            p(&t[1], "Hermite surface v tangent")?,
                            p(&n["m_iMixedDer"], "Hermite surface mixed derivative")?,
                        ])
                    })
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    nodes.len() == us.len() * vs.len(),
                    "Hermite surface node count"
                );
                // The envelope can pass the last node by a few thousandths:
                // there the end cell's cubic continues.
                Ok(Self::HermiteSurf {
                    envelope,
                    orient: s.fields["m_orientFlag"]
                        .as_bool()
                        .context("surface orientation")?,
                    us,
                    vs,
                    nodes,
                })
            }
            name => bail!("unsupported parametric surface {name}"),
        }
    }
    pub fn bounds(&self) -> [[f64; 2]; 2] {
        match self {
            Self::SurfRev { envelope, .. }
            | Self::RuledSurf { envelope, .. }
            | Self::HermiteSurf { envelope, .. } => *envelope,
        }
    }
    /// Second derivatives along u and along v of a Hermite surface; zero
    /// for the other surfaces, whose steps come from their profiles.
    pub(crate) fn second_derivatives(&self, u: f64, v: f64) -> (Point, Point) {
        match self {
            Self::HermiteSurf { us, vs, nodes, .. } => (
                hermite_patch(us, vs, nodes, u, v, 2, 0),
                hermite_patch(us, vs, nodes, u, v, 0, 2),
            ),
            _ => ([0.; 3], [0.; 3]),
        }
    }
    pub fn orientation(&self) -> bool {
        match self {
            Self::SurfRev { orient, .. }
            | Self::RuledSurf { orient, .. }
            | Self::HermiteSurf { orient, .. } => *orient,
        }
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Result<Point> {
        let e = self.bounds();
        ensure!(
            u >= e[0][0] - 1e-9
                && u <= e[1][0] + 1e-9
                && v >= e[0][1] - 1e-9
                && v <= e[1][1] + 1e-9,
            "surface coordinates outside envelope"
        );
        Ok(match self {
            Self::SurfRev {
                center,
                x,
                y,
                z,
                profile,
                ..
            } => {
                let q = profile.eval(v);
                let local = [
                    q[0] * u.cos() - q[1] * u.sin(),
                    q[0] * u.sin() + q[1] * u.cos(),
                    q[2],
                ];
                add(
                    *center,
                    add(add(mul(*x, local[0]), mul(*y, local[1])), mul(*z, local[2])),
                )
            }
            Self::RuledSurf {
                envelope: _,
                profile1,
                profile2,
                ..
            } => {
                let t = |c: &Curve| c_range(c)[0] + u * (c_range(c)[1] - c_range(c)[0]);
                add(
                    mul(profile1.eval(t(profile1)), 1. - v),
                    mul(profile2.eval(t(profile2)), v),
                )
            }
            Self::HermiteSurf { us, vs, nodes, .. } => hermite_patch(us, vs, nodes, u, v, 0, 0),
        })
    }
    /// Unit normal, oriented by the surface flag. Where the patch is
    /// singular (a side or corner collapsed to a point) the limit is taken
    /// from inside, stepping towards the centre of the envelope.
    pub fn normal(&self, u: f64, v: f64) -> Result<Point> {
        let first = self.normal_at(u, v);
        if !matches!(&first, Err(e) if e.to_string() == "surface normal is singular")
            || matches!(self, Self::SurfRev { .. })
        {
            return first;
        }
        let e = self.bounds();
        let c = [(e[0][0] + e[1][0]) / 2., (e[0][1] + e[1][1]) / 2.];
        for t in [1e-6, 1e-4, 1e-2] {
            if let Ok(n) = self.normal_at(u + (c[0] - u) * t, v + (c[1] - v) * t) {
                return Ok(n);
            }
        }
        first
    }
    fn normal_at(&self, u: f64, v: f64) -> Result<Point> {
        let e = self.bounds();
        ensure!(
            u.is_finite()
                && v.is_finite()
                && u >= e[0][0] - 1e-9
                && u <= e[1][0] + 1e-9
                && v >= e[0][1] - 1e-9
                && v <= e[1][1] + 1e-9,
            "coordinates outside envelope"
        );
        if let Self::SurfRev {
            x,
            y,
            z,
            profile,
            orient,
            ..
        } = self
        {
            let q = profile.eval(v);
            let dq = profile.derivative(v);
            let cu = u.cos();
            let su = u.sin();
            let ru = [-q[0] * su - q[1] * cu, q[0] * cu - q[1] * su, 0.];
            let rv = [dq[0] * cu - dq[1] * su, dq[0] * su + dq[1] * cu, dq[2]];
            let transform = |q: Point| add(add(mul(*x, q[0]), mul(*y, q[1])), mul(*z, q[2]));
            let tu = transform(ru);
            let tv = transform(rv);
            let mut n = cross(tu, tv);
            ensure!(
                n.iter().all(|value| value.is_finite()),
                "nonfinite surface normal"
            );
            if len(n) <= 1e-12 {
                // At a revolution pole, the U derivative collapses.  Use the
                // limiting radial/axial profile tangent instead.
                ensure!(
                    q[0].abs() <= 1e-9 && q[1].abs() <= 1e-9 && dq[1].abs() <= 1e-9,
                    "unsupported nonmeridional parametric singularity"
                );
                // Serialized meridian profiles have their radial tangent in
                // local X.  At the rotation axis the limiting normal is
                // continuous in U; projecting dq onto the current radial
                // direction would spuriously change the pole sign.
                let pole_du = transform([-su, cu, 0.]);
                let pole_dv = transform([dq[0] * cu, dq[0] * su, dq[2]]);
                n = cross(pole_du, pole_dv);
                ensure!(len(n) > 1e-12, "surface normal is singular");
            }
            return Ok(mul(n, if *orient { 1. / len(n) } else { -1. / len(n) }));
        }
        if let Self::HermiteSurf {
            us,
            vs,
            nodes,
            orient,
            ..
        } = self
        {
            let normal = |u, v| {
                cross(
                    hermite_patch(us, vs, nodes, u, v, 1, 0),
                    hermite_patch(us, vs, nodes, u, v, 0, 1),
                )
            };
            let n = normal(u, v);
            ensure!(
                n.iter().all(|value| value.is_finite()),
                "nonfinite surface normal"
            );
            let l = len(n);
            ensure!(l > 1e-12, "surface normal is singular");
            return Ok(mul(n, if *orient { 1. / l } else { -1. / l }));
        }
        let Self::RuledSurf {
            profile1,
            profile2,
            orient,
            ..
        } = self
        else {
            unreachable!()
        };
        let r1 = c_range(profile1);
        let r2 = c_range(profile2);
        let t1 = r1[0] + u * (r1[1] - r1[0]);
        let t2 = r2[0] + u * (r2[1] - r2[0]);
        let p1 = profile1.eval(t1);
        let p2 = profile2.eval(t2);
        let du = add(
            mul(profile1.derivative(t1), (1. - v) * (r1[1] - r1[0])),
            mul(profile2.derivative(t2), v * (r2[1] - r2[0])),
        );
        let dv = sub(p2, p1);
        let n = cross(du, dv);
        ensure!(
            n.iter().all(|value| value.is_finite()),
            "nonfinite surface normal"
        );
        let l = len(n);
        ensure!(l > 1e-12, "surface normal is singular");
        Ok(mul(n, if *orient { 1. / l } else { -1. / l }))
    }
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn c_range(c: &Curve) -> [f64; 2] {
    match c {
        Curve::Line { range, .. }
        | Curve::Arc { range, .. }
        | Curve::Hermite { range, .. }
        | Curve::Nurbs { range, .. } => *range,
    }
}
