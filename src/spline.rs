// Mechanically adapted from Wikipedia: Spline_interpolation#Algorithm_to_find_the_interpolating_cubic_spline

pub fn compute_spline_tangents(points: &Vec<[f32; 2]>) -> Vec<f32>
{
    let n = points.len();
    if n < 2
    {
        return vec![0.0; n];
    }
    
    let x: Vec<f32> = points.iter().map(|p| p[0]).collect();
    let y: Vec<f32> = points.iter().map(|p| p[1]).collect();
    
    let mut h = vec![0.0; n - 1];
    let mut k = vec![0.0; n];
    
    for i in 0..n - 1
    {
        h[i] = x[i + 1] - x[i];
    }
    
    let mut alpha = vec![0.0; n - 1];
    for i in 1..n - 1
    {
        alpha[i] = 3.0 * ((y[i + 1] - y[i]) / h[i] - (y[i] - y[i - 1]) / h[i - 1]);
    }
    
    let mut l = vec![0.0; n];
    let mut mu = vec![0.0; n - 1];
    let mut z = vec![0.0; n];
    
    l[0] = 1.0;
    mu[0] = 0.0;
    z[0] = 0.0;
    
    for i in 1..n - 1
    {
        l[i] = 2.0 * (x[i + 1] - x[i - 1]) - h[i - 1] * mu[i - 1];
        mu[i] = h[i] / l[i];
        z[i] = (alpha[i] - h[i - 1] * z[i - 1]) / l[i];
    }
    
    l[n - 1] = 1.0;
    z[n - 1] = 0.0;
    
    for j in (0..n - 1).rev()
    {
        k[j] = z[j] - mu[j] * k[j + 1];
    }
    
    k[n - 1] = z[n - 1];
    
    k
}
pub fn binary_search_last_lt(sorted_nodes: &[[f32; 2]], r: f32) -> usize
{
    match sorted_nodes.binary_search_by(|node| node[0].partial_cmp(&r).unwrap())
    {
        Ok(index) => index.saturating_sub(1),
        Err(index) => index.saturating_sub(1),
    }
}

pub fn interpolate_spline(x: f32, sorted_nodes: &Vec<[f32; 2]>, tangents: &Vec<f32>, i: usize) -> f32 {
    let x0 = sorted_nodes[i][0];
    let y0 = sorted_nodes[i][1];
    let x1 = sorted_nodes[i + 1][0];
    let y1 = sorted_nodes[i + 1][1];
    
    let t0 = tangents[i];
    let t1 = tangents[i + 1];
    
    let h = x1 - x0;
    let i0 = (x - x0) / h;
    let i1 = 1.0 - i0;
    
    y0 * i1 + y1 * i0
    + (t0 * (i1+1.0) + t1 * (i0+1.0))
      * (-h*h*i0*i1 * (1.0/3.0))
}
