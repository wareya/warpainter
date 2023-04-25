
#[derive(Clone, Debug)]
pub (crate) struct Transform
{
    pub (crate) rows : [[f32; 3]; 3],
}
impl Default for Transform
{
    fn default() -> Self
    {
        Self::ident()
    }
}
impl<'a, 'b> core::ops::Mul<&'b Transform> for &'a Transform
{
    type Output = Transform;
    fn mul(self, other : &'b Transform) -> Transform
    {
        let mut out = Transform::zero();
        #[allow(clippy::needless_range_loop)]
        for row in 0..3
        {
            for col in 0..3
            {
                out.rows[row][col] = 0.0;
                for i in 0..3
                {
                    out.rows[row][col] += self.rows[row][i] * other.rows[i][col];
                }
            }
        }
        out
    }
}
impl<'a, 'b> core::ops::Mul<&'b [f32; 2]> for &'a Transform
{
    type Output = [f32; 2];
    fn mul(self, other : &'b [f32; 2]) -> [f32; 2]
    {
        let other = [other[0], other[1], 1.0];
        let mut out = [0.0, 0.0, 0.0];
        #[allow(clippy::needless_range_loop)]
        for row in 0..3
        {
            for col in 0..3
            {
                out[row] += self.rows[row][col] * other[col];
            }
        }
        [out[0], out[1]]
    }
}

pub (crate) fn xform_points(xform : &Transform, points : &mut [[f32; 2]])
{
    for point in points.iter_mut()
    {
        *point = xform * &*point;
    }
}

impl Transform {
    pub (crate) fn zero() -> Self
    {
        Self { rows : [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]] }
    }
    pub (crate) fn ident() -> Self
    {
        Self { rows : [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] }
    }
    pub (crate) fn get_translation(&self) -> [f32; 2]
    {
        [self.rows[0][2], self.rows[1][2]]
    }
    // FIXME make a vector
    pub (crate) fn get_scale(&self) -> f32
    {
        let a = self.rows[0][0];
        let b = self.rows[0][1];
        let c = self.rows[1][0];
        let d = self.rows[1][1];
        
        let x = length(&[a, c]);
        let y = length(&[b, d]);
        x/2.0 + y/2.0
    }
    pub (crate) fn get_rotation(&self) -> f32
    {
        let mut d = self.clone();
        d.rows[0][2] = 0.0;
        d.rows[1][2] = 0.0;
        d.set_scale(1.0);
        
        let r = &d * &[1.0, 0.0];
        
        let psi = (r[1]).atan2(r[0]);
        
        psi / core::f32::consts::PI * 180.0
    }
    pub (crate) fn translate(&mut self, translation : [f32; 2])
    {
        let mut other = Self::ident();
        other.rows[0][2] = translation[0];
        other.rows[1][2] = translation[1];
        
        let new = &other * &*self;
        self.rows = new.rows;
    }
    // FIXME make a vector
    pub (crate) fn scale(&mut self, scale : f32)
    {
        let mut other = Self::ident();
        other.rows[0][0] = scale;
        other.rows[1][1] = scale;
        
        let new = &other * &*self;
        self.rows = new.rows;
    }
    pub (crate) fn set_scale(&mut self, scale : f32)
    {
        let old_scale = self.get_scale();
        if old_scale > 0.0
        {
            self.scale(1.0 / old_scale);
        }
        self.scale(scale);
    }
    pub (crate) fn rotate(&mut self, angle : f32)
    {
        let mut other = Self::ident();
        let _cos = (angle * core::f32::consts::PI / 180.0).cos();
        let _sin = (angle * core::f32::consts::PI / 180.0).sin();
        other.rows[0][0] =  _cos;
        other.rows[0][1] = -_sin;
        other.rows[1][0] =  _sin;
        other.rows[1][1] =  _cos;
        
        let new = &other * &*self;
        self.rows = new.rows;
    }
    pub (crate) fn make_uniform(&mut self)
    {
        let _other = Self::ident();
        // FIXME / TODO
    }
    pub (crate) fn inverse(&self) -> Self
    {
        let mut m = [[0.0f64; 3]; 3];
        
        #[allow(clippy::needless_range_loop)]
        for y in 0..3
        {
            for x in 0..3
            {
                m[x][y] = self.rows[x][y] as f64;
            }
        }
        // computes the inverse of a matrix m
        let det = m[0][0] * (m[1][1] * m[2][2] - m[2][1] * m[1][2]) -
                  m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) +
                  m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

        let invdet = 1.0 / det;

        let mut inverted = [[0.0f32; 3]; 3];
        
        inverted[0][0] = ((m[1][1] * m[2][2] - m[2][1] * m[1][2]) * invdet) as f32;
        inverted[0][1] = ((m[0][2] * m[2][1] - m[0][1] * m[2][2]) * invdet) as f32;
        inverted[0][2] = ((m[0][1] * m[1][2] - m[0][2] * m[1][1]) * invdet) as f32;
        inverted[1][0] = ((m[1][2] * m[2][0] - m[1][0] * m[2][2]) * invdet) as f32;
        inverted[1][1] = ((m[0][0] * m[2][2] - m[0][2] * m[2][0]) * invdet) as f32;
        inverted[1][2] = ((m[1][0] * m[0][2] - m[0][0] * m[1][2]) * invdet) as f32;
        inverted[2][0] = ((m[1][0] * m[2][1] - m[2][0] * m[1][1]) * invdet) as f32;
        inverted[2][1] = ((m[2][0] * m[0][1] - m[0][0] * m[2][1]) * invdet) as f32;
        inverted[2][2] = ((m[0][0] * m[1][1] - m[1][0] * m[0][1]) * invdet) as f32;
        
        Self { rows : inverted }
    }
}

pub (crate) fn length_sq(vec : &[f32]) -> f32
{
    let mut r = 0.0;
    for x in vec.iter()
    {
        r += x*x;
    }
    r
}

pub (crate) fn length(vec : &[f32]) -> f32
{
    length_sq(vec).sqrt()
}

pub (crate) fn lerp(from : f32, to : f32, amount : f32) -> f32
{
    from * (1.0-amount) + to * amount
}

pub (crate) fn vec_lerp<const N: usize>(from : &[f32; N], to : &[f32; N], amount : f32) -> [f32; N]
{
    let mut out = [0.0; N];
    for i in 0..N
    {
        out[i] = lerp(from[i], to[i], amount);
    }
    out
}

pub (crate) fn vec_eq<const N: usize>(a : &[f32; N], b : &[f32; N]) -> bool
{
    for i in 0..N
    {
        if a[i] != b[i]
        {
            return false;
        }
    }
    true
}

pub (crate) fn vec_sub<const N: usize>(from : &[f32; N], to : &[f32; N]) -> [f32; N]
{
    let mut out = [0.0; N];
    for i in 0..N
    {
        out[i] = from[i] - to[i];
    }
    out
}

pub (crate) fn vec_add
    <const N: usize,
     T : core::ops::Add<Output = T> + std::marker::Copy + Default>
    (from : &[T; N], to : &[T; N]) -> [T; N]
{
    let mut out = [T::default(); N];
    for i in 0..N
    {
        out[i] = from[i] + to[i];
    }
    out
}
pub (crate) fn vec_dot
    <const N: usize,
     T : core::ops::Add<Output = T> + std::marker::Copy + Default + std::ops::Mul<Output = T>>
    (a : &[T; N], b : &[T; N]) -> T
{
    let mut out = T::default();
    for i in 0..N
    {
        out = out + a[i] * b[i];
    }
    out
}

pub (crate) fn vec_floor<const N: usize>(a : &[f32; N]) -> [f32; N]
{
    let mut out = [0.0; N];
    for i in 0..N
    {
        out[i] = a[i].floor();
    }
    out
}

pub (crate) fn rect_enclose_point(mut rect : [[f32; 2]; 2], point : [f32; 2]) -> [[f32; 2]; 2]
{
    rect[0][0] = rect[0][0].min(point[0]);
    rect[0][1] = rect[0][1].min(point[1]);
    rect[1][0] = rect[1][0].max(point[0]);
    rect[1][1] = rect[1][1].max(point[1]);
    rect
}
pub (crate) fn rect_enclose_rect(mut rect : [[f32; 2]; 2], rect_2 : [[f32; 2]; 2]) -> [[f32; 2]; 2]
{
    rect = rect_enclose_point(rect, rect_2[0]);
    rect = rect_enclose_point(rect, rect_2[1]);
    rect
}
pub (crate) fn rect_normalize(rect : [[f32; 2]; 2]) -> [[f32; 2]; 2]
{
    rect_enclose_point([rect[0], rect[0]], rect[1])
}
pub (crate) fn rect_translate(rect : [[f32; 2]; 2], vec : [f32; 2]) -> [[f32; 2]; 2]
{
    [[rect[0][0] + vec[0], rect[0][1] + vec[1]], [rect[1][0] + vec[0], rect[1][1] + vec[1]]]
}