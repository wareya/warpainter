#[inline]
fn lerp(a : f32, b : f32, t : f32) -> f32
{
    a * (1.0 - t) + b * t
}
#[inline]
pub (crate) fn unlerp(a : f32, b : f32, t : f32) -> f32
{
    if a == b { return 0.0; }
    (t - a) / (b - a)
}

#[inline]
// return what the output alpha should be given the two input alpha values
pub (crate) fn alpha_combine(a : f32, b : f32) -> f32
{
    b * (1.0 - a) + a
}

#[inline]
pub (crate) fn px_lerp_float<const N : usize>(a : [f32; N], b : [f32; N], amount : f32) -> [f32; N]
{
    let mut r = [0.0; N];
    for i in 0..N
    {
        r[i] = lerp(b[i], a[i], amount);
    }
    r
}
#[inline]
pub (crate) fn px_lerp(a : [u8; 4], b : [u8; 4], amount : f32) -> [u8; 4]
{
    px_to_int(px_lerp_float(px_to_float(a), px_to_float(b), amount))
}

#[inline]
pub (crate) fn px_lerp_biased_float(a : [f32; 4], b : [f32; 4], amount : f32, _modifier : f32, _flag : bool) -> [f32; 4]
{
    let total_a = lerp(b[3], a[3], amount);
    
    if total_a == 0.0
    {
        return [a[0], a[1], a[2], 0.0];
    }
    
    let mut r = [0.0; 4];
    
    for i in 0..3
    {
        r[i] = lerp(b[i] * b[3], a[i] * a[3], amount) / total_a;
    }
    r[3] = total_a;
    
    r
}
#[inline]
pub (crate) fn px_lerp_biased(a : [u8; 4], b : [u8; 4], amount : f32, _modifier : f32, _flag : bool) -> [u8; 4]
{
    px_to_int(px_lerp_biased_float(px_to_float(a), px_to_float(b), amount, _modifier, _flag))
}


pub (crate) struct BlendModeHardMix;
impl BlendModeSimpleExtra for BlendModeHardMix
{
    fn blend(top : f32, bottom : f32, _opacity : f32, fill : f32) -> f32
    {
        // Photoshop doesn't actually use a threshold, but rather an alpha-sensitive scaling formula that ensures that white+white remains pinned near white and black+black remains pinned near black.
        let mut n = bottom + top*fill;
        n -= (fill*0.5/255.0f32).copysign(top-0.5); // rounding hack
        n -= 0.5;
        n -= fill*0.5;
        n /= 1.0 - fill;
        n += 0.5;
        n.clamp(0.0, 1.0)
    }
}
#[inline]
pub (crate) fn px_func_extra_float<T : BlendModeSimpleExtra>
    (mut a : [f32; 4], b : [f32; 4], amount : f32, mut modifier : f32, funny_flag : bool)
    -> [f32; 4]
{
    if funny_flag
    {
        modifier *= a[3];
        a[3] = amount;
    }
    else
    {
        a[3] *= amount;
    }
    
    let mut r = [0.0; 4];
    
    // a is top layer, b is bottom
    let b_under_a = b[3] * (1.0 - a[3] * modifier);
    r[3] = a[3] * modifier + b_under_a;
    
    let m = 1.0 / r[3];
    
    {
        for i in 0..3
        {
            let mut n = T::blend(a[i], b[i], a[3], modifier);
            n = lerp(b[i], n, a[3]);
            n = lerp(a[i], n, b[3] * m);
            r[i] = n;
        }
    }
    
    r
}

#[inline]
pub (crate) fn px_func_extra<T : BlendModeSimpleExtra>
    (a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32, funny_flag : bool)
    -> [u8; 4]
{
    if a[3] == 0 || amount == 0.0 || modifier == 0.0
    {
        return b;
    }
    else if b[3] == 0
    {
        return [a[0], a[1], a[2], to_int(to_float(a[3]) * amount * modifier)];
    }

    // a is top layer, b is bottom
    px_to_int(px_func_extra_float::<T>(px_to_float(a), px_to_float(b), amount, modifier, funny_flag))
}



#[inline]
pub (crate) fn px_func_float<T : BlendModeSimple>
    (mut a : [f32; 4], b : [f32; 4], amount : f32, modifier : f32, _funny_flag : bool)
    -> [f32; 4]
{
    a[3] *= amount;
    a[3] *= modifier;
    
    if a[3] == 0.0
    {
        return b;
    }
    else if b[3] == 0.0
    {
        return a;
    }

    let mut r = [0.0; 4];
    
    // a is top layer, b is bottom
    let b_under_a = b[3] * (1.0 - a[3]);
    r[3] = b_under_a + a[3];
    let m = 1.0 / r[3];
    
    let b_a = b_under_a * m;
    let a_a = a[3] * m;
    
    for i in 0..3
    {
        r[i] = lerp(a[i], T::blend(a[i], b[i]), b[3]) * a_a + b[i] * b_a;
        //r[i] = T::blend(a[i], b[i]);
    }
    
    r
}
#[inline]
pub (crate) fn px_func<T : BlendModeSimple>
    (a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32, funny_flag : bool)
    -> [u8; 4]
{
    if a[3] == 0 || amount == 0.0 || modifier == 0.0
    {
        return b;
    }
    else if b[3] == 0
    {
        return [a[0], a[1], a[2], to_int(to_float(a[3]) * amount * modifier)];
    }

    // a is top layer, b is bottom
    px_to_int(px_func_float::<T>(px_to_float(a), px_to_float(b), amount, modifier, funny_flag))
}


pub (crate) trait BlendModeSimple
{
    fn blend(top : f32, bottom : f32) -> f32;
}

pub (crate) struct BlendModeNormal;
impl BlendModeSimple for BlendModeNormal
{
    fn blend(top : f32, _bottom : f32) -> f32
    {
        top
    }
}
pub (crate) struct BlendModeMultiply;
impl BlendModeSimple for BlendModeMultiply
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        bottom * top
    }
}
pub (crate) struct BlendModeDivide;
impl BlendModeSimple for BlendModeDivide
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom / top).clamp(0.0, 1.0) // dst/src
    }
}
pub (crate) struct BlendModeScreen;
impl BlendModeSimple for BlendModeScreen
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        1.0 - ((1.0 - bottom) * (1.0 - top)).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeAdd;
impl BlendModeSimple for BlendModeAdd
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom + top).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeAddGlow;
impl BlendModeSimple for BlendModeAddGlow
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        bottom + top
    }
}
pub (crate) struct BlendModeSubtract;
impl BlendModeSimple for BlendModeSubtract
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom - top).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeDifference;
impl BlendModeSimple for BlendModeDifference
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom - top).abs()
    }
}
pub (crate) struct BlendModeSignedDifference;
impl BlendModeSimple for BlendModeSignedDifference
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom - top)*0.5 + 0.5
    }
}
pub (crate) struct BlendModeSignedAdd;
impl BlendModeSimple for BlendModeSignedAdd
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom + top*2.0 - 1.0).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeNegation;
impl BlendModeSimple for BlendModeNegation
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        1.0 - (1.0 - top - bottom).abs().clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeLighten; // FIXME: lighter color too
impl BlendModeSimple for BlendModeLighten
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        top.max(bottom)
    }
}
pub (crate) struct BlendModeDarken; // FIXME: darker color too
impl BlendModeSimple for BlendModeDarken
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        top.min(bottom)
    }
}
pub (crate) struct BlendModeLinearBurn;
impl BlendModeSimple for BlendModeLinearBurn
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom + top - 1.0).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeColorBurn;
impl BlendModeSimple for BlendModeColorBurn
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (1.0 - ((1.0 - bottom) / top)).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeColorDodge;
impl BlendModeSimple for BlendModeColorDodge
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom / (1.0 - top)).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeGlow;
impl BlendModeSimple for BlendModeGlow
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (top * top / (1.0 - bottom)).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeReflect;
impl BlendModeSimple for BlendModeReflect
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom * bottom / (1.0 - top)).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeOverlay;
impl BlendModeSimple for BlendModeOverlay
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        BlendModeHardLight::blend(bottom, top)
    }
}
pub (crate) struct BlendModeSoftLight;
impl BlendModeSimple for BlendModeSoftLight
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        if top < 0.5
        {
            2.0*bottom*top + bottom*bottom*(1.0-2.0*top)
        }
        else
        {
            2.0*bottom*(1.0-top) + bottom.sqrt()*(2.0*top-1.0)
        }
    }
}
pub (crate) struct BlendModeHardLight;
impl BlendModeSimple for BlendModeHardLight
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        if top < 0.5
        {
            top * 2.0 * bottom
        }
        else
        {
            1.0 - 2.0 * (1.0 - top) * (1.0 - bottom)
        }
    }
}
pub (crate) struct BlendModeVividLight;
impl BlendModeSimple for BlendModeVividLight
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        if top < 0.0000001 && bottom == 1.0
        {
            if bottom >= 1.0
            {
                1.0
            }
            else
            {
                0.0
            }
        }
        else if top == 1.0
        {
            if bottom <= 0.0
            {
                0.0
            }
            else
            {
                1.0
            }
        }
        else if top < 0.5
        {
            (1.0 - (1.0 - bottom) / (2.0 * top)).clamp(0.0, 1.0)
        }
        else
        {
            (bottom / (2.0 * (1.0 - top))).clamp(0.0, 1.0)
        }
    }
}
pub (crate) struct BlendModeLinearLight;
impl BlendModeSimple for BlendModeLinearLight
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        ((2.0 * top + bottom) - 1.0).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModePinLight;
impl BlendModeSimple for BlendModePinLight
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        bottom.min(2.0 * top).max(2.0 * top - 1.0).clamp(0.0, 1.0)
    }
}
pub (crate) struct BlendModeExclusion;
impl BlendModeSimple for BlendModeExclusion
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        (bottom + top - (2.0 * (top * bottom))).clamp(0.0, 1.0)
    }
}



#[inline]
pub (crate) fn px_func_triad_float<T : BlendModeTriad>(mut a : [f32; 4], b : [f32; 4], amount : f32, _modifier : f32, _flag : bool) -> [f32; 4]
{
    a[3] *= amount;
    
    if a[3] == 0.0 || amount == 0.0
    {
        return b;
    }
    else if b[3] == 0.0
    {
        return [a[0], a[1], a[2], a[3]];
    }

    let mut r = [0.0; 4];
    
    // a is top layer, b is bottom
    let b_under_a = b[3] * (1.0 - a[3]);
    r[3] = a[3] + b_under_a;
    let m = 1.0 / r[3];
    
    let a_a = a[3] * m;
    let b_a = b_under_a * m;
    
    let a_triad : [f32; 3] = [a[0], a[1], a[2]];
    let b_triad : [f32; 3] = [b[0], b[1], b[2]];
    
    let r_triad = T::blend(a_triad, b_triad);
    
    r[0] = r_triad[0] * a_a + b[0] * b_a;
    r[1] = r_triad[1] * a_a + b[1] * b_a;
    r[2] = r_triad[2] * a_a + b[2] * b_a;
    
    r
    
}
#[inline]
pub (crate) fn px_func_triad<T : BlendModeTriad>(a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32, flag : bool) -> [u8; 4]
{
    px_to_int(px_func_triad_float::<T>(px_to_float(a), px_to_float(b), amount, modifier, flag))
}

pub (crate) trait BlendModeTriad
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3];
}


// functions to implement SVG-style Hue/Sat/Color/etc blend modes
pub (crate) fn calc_y(rgb : [f32; 3]) -> f32
{
    rgb[0] * 0.3 + rgb[1] * 0.59 + rgb[2] * 0.11
}
pub (crate) fn color_clipped(rgb : [f32; 3]) -> [f32; 3]
{
    let y = calc_y(rgb);
    let low  = rgb[0].min(rgb[1]).min(rgb[2]);
    let high = rgb[0].max(rgb[1]).max(rgb[2]);
    
    // calculate amount of overshoot
    let f = if low < 0.0
    {
        1.0 - y / (y - low)
    }
    else if high > 1.0
    {
        1.0 - (1.0 - y) / (high - y)
    }
    else
    {
        0.0
    };
    
    // lerp towards gray to prevent overshoot
    [lerp(rgb[0], y, f), lerp(rgb[1], y, f), lerp(rgb[2], y, f)]
}
pub (crate) fn apply_y(rgb : [f32; 3], y : f32) -> [f32; 3]
{
    let delta = y - calc_y(rgb);
    color_clipped([rgb[0] + delta, rgb[1] + delta, rgb[2] + delta])
}
pub (crate) fn calc_sat(rgb : [f32; 3]) -> f32
{
    let low  = rgb[0].min(rgb[1]).min(rgb[2]);
    let high = rgb[0].max(rgb[1]).max(rgb[2]);
    high - low
}
pub (crate) fn apply_sat_and_y(rgb : [f32; 3], sat : f32, y : f32) -> [f32; 3]
{
    fn lowest(v : &[f32]) -> usize
    {
        v.iter().enumerate().min_by(|(_, a), (_, b)| a.total_cmp(b)).map(|(i, _)| i).unwrap()
    }
    fn highest(v : &[f32]) -> usize
    {
        v.iter().enumerate().max_by(|(_, a), (_, b)| a.total_cmp(b)).map(|(i, _)| i).unwrap()
    }
    let i_low = lowest(&rgb);
    let i_high = highest(&rgb);
    let mut i_mid = 0;
    while i_low == i_mid || i_high == i_mid
    {
        i_mid += 1;
    }
    
    if rgb[i_high] == rgb[i_low]
    {
        [0.0, 0.0, 0.0]
    }
    else
    {
        let mut temp_rgb = [0.0, 0.0, 0.0];
        
        temp_rgb[i_low] = 0.0;
        temp_rgb[i_mid] = sat * (rgb[i_mid] - rgb[i_low]) / (rgb[i_high] - rgb[i_low]);
        temp_rgb[i_high] = sat;
        
        apply_y(temp_rgb, y)
    }
}

// SVG style

pub (crate) struct BlendModeHue;
impl BlendModeTriad for BlendModeHue
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        apply_sat_and_y(top, calc_sat(bottom), calc_y(bottom))
    }
}
pub (crate) struct BlendModeSaturation;
impl BlendModeTriad for BlendModeSaturation
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        apply_sat_and_y(bottom, calc_sat(top), calc_y(bottom))
    }
}
pub (crate) struct BlendModeColor;
impl BlendModeTriad for BlendModeColor
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        apply_y(top, calc_y(bottom))
    }
}
pub (crate) struct BlendModeLuminosity;
impl BlendModeTriad for BlendModeLuminosity
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        apply_y(bottom, calc_y(top))
    }
}

// "soft" versions, using HSV

pub (crate) struct BlendModeFlatHue;
impl BlendModeTriad for BlendModeFlatHue
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsva_top    = rgb_to_hsv([top[0], top[1], top[2], 1.0]);
        let hsva_bottom = rgb_to_hsv([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsv_to_rgb([hsva_top[0], hsva_bottom[1], hsva_bottom[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}
pub (crate) struct BlendModeFlatSaturation;
impl BlendModeTriad for BlendModeFlatSaturation
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsva_top    = rgb_to_hsv([top[0], top[1], top[2], 1.0]);
        let hsva_bottom = rgb_to_hsv([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsv_to_rgb([hsva_bottom[0], hsva_top[1], hsva_bottom[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}
pub (crate) struct BlendModeFlatColor;
impl BlendModeTriad for BlendModeFlatColor
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsva_top    = rgb_to_hsv([top[0], top[1], top[2], 1.0]);
        let hsva_bottom = rgb_to_hsv([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsv_to_rgb([hsva_top[0], hsva_top[1], hsva_bottom[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}
pub (crate) struct BlendModeValue;
impl BlendModeTriad for BlendModeValue
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsva_top    = rgb_to_hsv([top[0], top[1], top[2], 1.0]);
        let hsva_bottom = rgb_to_hsv([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsv_to_rgb([hsva_bottom[0], hsva_bottom[1], hsva_top[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}

// "hard" versions, using HSL

// no difference between HSL and HSV hue filters
pub (crate) struct BlendModeHardSaturation;
impl BlendModeTriad for BlendModeHardSaturation
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsla_top    = rgb_to_hsl([top[0], top[1], top[2], 1.0]);
        let hsla_bottom = rgb_to_hsl([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsl_to_rgb([hsla_bottom[0], hsla_top[1], hsla_bottom[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}
pub (crate) struct BlendModeHardColor;
impl BlendModeTriad for BlendModeHardColor
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsla_top    = rgb_to_hsl([top[0], top[1], top[2], 1.0]);
        let hsla_bottom = rgb_to_hsl([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsl_to_rgb([hsla_top[0], hsla_top[1], hsla_bottom[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}
pub (crate) struct BlendModeLightness;
impl BlendModeTriad for BlendModeLightness
{
    fn blend(top : [f32; 3], bottom : [f32; 3]) -> [f32; 3]
    {
        let hsla_top    = rgb_to_hsl([top[0], top[1], top[2], 1.0]);
        let hsla_bottom = rgb_to_hsl([bottom[0], bottom[1], bottom[2], 1.0]);
        let rgba = hsl_to_rgb([hsla_bottom[0], hsla_bottom[1], hsla_top[2], 1.0]);
        [rgba[0], rgba[1], rgba[2]]
    }
}


#[inline]
pub (crate) fn px_func_full_float<T : BlendModeFull>(a : [f32; 4], b : [f32; 4], amount : f32, modifier : f32, _flag : bool) -> [f32; 4]
{
    T::blend(a, b, amount * modifier)
}
#[inline]
pub (crate) fn px_func_full<T : BlendModeFull>(a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32, flag : bool) -> [u8; 4]
{
    px_to_int(px_func_full_float::<T>(px_to_float(a), px_to_float(b), amount, modifier, flag))
}
pub (crate) trait BlendModeFull
{
    fn blend(top : [f32; 4], bottom : [f32; 4], amount : f32) -> [f32; 4];
}

pub (crate) trait BlendModeExtra
{
    fn blend(top : [f32; 4], bottom : [f32; 4], amount : f32, modifier : f32) -> [f32; 4];
}

pub (crate) trait BlendModeSimpleExtra
{
    fn blend(top : f32, bottom : f32, amount : f32, modifier : f32) -> f32;
}

pub (crate) struct BlendModeUnder;
impl BlendModeFull for BlendModeUnder
{
    fn blend(mut a : [f32; 4], mut b : [f32; 4], amount : f32) -> [f32; 4]
    {
        a[3] *= amount;
        
        if a[3] == 0.0
        {
            return b;
        }
        else if b[3] == 0.0
        {
            return a;
        }
        
        let mut r = [0.0; 4];
        
        std::mem::swap(&mut a, &mut b);
        
        let b_under_a = b[3] * (1.0 - a[3]);
        r[3] = b_under_a + a[3];
        let m = 1.0 / (r[3]);
        
        let a_a = a[3] * m;
        let b_a = b_under_a * m;
        
        for i in 0..3
        {
            r[i] = a[i] * a_a + b[i] * b_a;
        }
        
        r
    }
}
pub (crate) struct BlendModeComposite;
impl BlendModeFull for BlendModeComposite
{
    fn blend(mut a : [f32; 4], b : [f32; 4], amount : f32) -> [f32; 4]
    {
        a[3] *= amount;
        
        if a[3] == 0.0
        {
            return b;
        }
        else if b[3] == 0.0
        {
            return a;
        }
        
        let mut r = [0.0; 4];
        
        let b_under_a = b[3] * (1.0 - a[3]);
        r[3] = b_under_a + a[3];
        let m = 1.0 / (r[3]);
        
        let a_a = a[3] * m;
        let b_a = b_under_a * m;
        
        for i in 0..3
        {
            r[i] = a[i] * a_a + b[i] * b_a;
            //r[i] = lerp(a[i], b[i], a[3]) * a_a + b[i] * b_a;
        }
        
        r
    }
}
pub (crate) struct BlendModeErase;
impl BlendModeFull for BlendModeErase
{
    fn blend(top : [f32; 4], mut bottom : [f32; 4], amount : f32) -> [f32; 4]
    {
        bottom[3] = lerp(bottom[3], bottom[3] * (1.0 - top[3]), amount);
        bottom
    }
}
pub (crate) struct BlendModeReveal;
impl BlendModeFull for BlendModeReveal
{
    fn blend(top : [f32; 4], mut bottom : [f32; 4], amount : f32) -> [f32; 4]
    {
        bottom[3] = lerp(bottom[3], 1.0 - (1.0 - bottom[3]) * (1.0 - top[3]), amount);
        bottom
    }
}
pub (crate) struct BlendModeAlphaMask;
impl BlendModeFull for BlendModeAlphaMask
{
    fn blend(top : [f32; 4], mut bottom : [f32; 4], amount : f32) -> [f32; 4]
    {
        let l = (top[0] + top[1] + top[2]) * (1.0/3.0);
        bottom[3] = lerp(bottom[3], bottom[3] * l, amount * top[3]);
        bottom
    }
}
pub (crate) struct BlendModeAlphaReject;
impl BlendModeFull for BlendModeAlphaReject
{
    fn blend(top : [f32; 4], mut bottom : [f32; 4], amount : f32) -> [f32; 4]
    {
        let l = 1.0 - (top[0] + top[1] + top[2]) * (1.0/3.0);
        bottom[3] = lerp(bottom[3], bottom[3] * l, amount * top[3]);
        bottom
    }
}

pub (crate) struct BlendModeGlowDodge;
impl BlendModeFull for BlendModeGlowDodge
{
    fn blend(mut a : [f32; 4], b : [f32; 4], amount : f32) -> [f32; 4]
    {
        fn glow_dodge(a : f32, b : f32, alpha : f32, lower_alpha : f32) -> f32
        {
            let dodge = (b / (1.0 - a*alpha)).clamp(0.0, 1.0);
            lerp(dodge, a, 1.0 - lower_alpha)
        }
        a[3] = a[3] * amount;
        
        let over = a[3] + b[3] * (1.0 - a[3]);
        
        let mut blend = b;

        if over != 0.0
        {
            blend[0] = glow_dodge(a[0], b[0], a[3], b[3]);
            blend[1] = glow_dodge(a[1], b[1], a[3], b[3]);
            blend[2] = glow_dodge(a[2], b[2], a[3], b[3]);
        }
        blend[3] = over;

        blend
    }
}

type FloatBlendFn = dyn Fn([f32; 4], [f32; 4], f32, f32, bool) -> [f32; 4];
type IntBlendFn = fn([u8; 4], [u8; 4], f32, f32, bool) -> [u8; 4];

pub (crate) fn find_blend_func_float(blend_mode : &str) -> Box<FloatBlendFn>
{
    Box::new(match blend_mode
    {
        "Composite" => px_func_full_float::<BlendModeComposite>,
        
        "Multiply" => px_func_float::<BlendModeMultiply>,
        "Divide" => px_func_float::<BlendModeDivide>,
        "Screen" => px_func_float::<BlendModeScreen>,
        "Add" => px_func_float::<BlendModeAdd>,
        "Glow Add" => px_func_float::<BlendModeAddGlow>,
        "Subtract" => px_func_float::<BlendModeSubtract>,
        "Difference" => px_func_float::<BlendModeDifference>,
        "Signed Diff" => px_func_float::<BlendModeSignedDifference>,
        "Signed Add" => px_func_float::<BlendModeSignedAdd>,
        "Negation" => px_func_float::<BlendModeNegation>,
        "Lighten" => px_func_float::<BlendModeLighten>,
        "Darken" => px_func_float::<BlendModeDarken>,
        "Linear Burn" => px_func_float::<BlendModeLinearBurn>,
        "Color Burn" => px_func_float::<BlendModeColorBurn>,
        "Color Dodge" => px_func_float::<BlendModeColorDodge>,
        
        "Glow Dodge" => px_func_full_float::<BlendModeGlowDodge>,
        
        "Glow" => px_func_float::<BlendModeGlow>,
        "Reflect" => px_func_float::<BlendModeReflect>,
        "Overlay" => px_func_float::<BlendModeOverlay>,
        "Soft Light" => px_func_float::<BlendModeSoftLight>,
        "Hard Light" => px_func_float::<BlendModeHardLight>,
        "Vivid Light" => px_func_float::<BlendModeVividLight>,
        "Linear Light" => px_func_float::<BlendModeLinearLight>,
        "Pin Light" => px_func_float::<BlendModePinLight>,
        "Hard Mix" => px_func_extra_float::<BlendModeHardMix>,
        "Exclusion" => px_func_float::<BlendModeExclusion>,
        
        "Hue" => px_func_triad_float::<BlendModeHue>,
        "Saturation" => px_func_triad_float::<BlendModeSaturation>,
        "Color" => px_func_triad_float::<BlendModeColor>,
        "Luminosity" => px_func_triad_float::<BlendModeLuminosity>,
        
        "Flat Hue" => px_func_triad_float::<BlendModeFlatHue>,
        "Flat Sat" => px_func_triad_float::<BlendModeFlatSaturation>,
        "Flat Color" => px_func_triad_float::<BlendModeFlatColor>,
        "Value" => px_func_triad_float::<BlendModeValue>,
        
        "Hard Sat" => px_func_triad_float::<BlendModeHardSaturation>,
        "Hard Color" => px_func_triad_float::<BlendModeHardColor>,
        "Lightness" => px_func_triad_float::<BlendModeLightness>,
        
        "Erase" => px_func_full_float::<BlendModeErase>,
        "Reveal" => px_func_full_float::<BlendModeReveal>,
        "Alpha Mask" => px_func_full_float::<BlendModeAlphaMask>,
        "Alpha Reject" => px_func_full_float::<BlendModeAlphaReject>,
        "Under" => px_func_full_float::<BlendModeUnder>,
        
        "Interpolate" => px_lerp_biased_float,
        "Hard Interpolate" => |a, b, amount, _m, _u| px_lerp_float(a, b, amount * a[3]),
        
        "Clamp Erase" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], b[3].min(1.0 - a[3])], // used internally
        "Merge Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], a[3] * b[3]], // used internally
        "Clip Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], a[3].min(b[3])], // used internally
        "Max Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], a[3].max(b[3])], // used internally
        "Copy Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], a[3]], // used internally
        "Copy" => |a, _b, amount, _modifier, _unused| [a[0], a[1], a[2], a[3] * amount], // used internally
        
        "Dither" => |mut a, b, _amount, _modifier, flag|
        {
            // normal blending, but ignore amount and top alpha (handled by post func)
            a[3] = 1.0;
            px_func_float::<BlendModeNormal>(a, b, 1.0, 1.0, flag)
        },
        
        "Weld Under" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func_full_float::<BlendModeUnder>(a, b, amount, modifier, flag);
            out[3] = (a[3]*amount + b[3]).clamp(0.0, 1.0);
            out
        },
        
        //FIXME
        // Alpha Antiblend
        // Blend Weld
        
        "Sum Weld" => |a, b, amount, _modifier, _flag|
        {
            if a[3] == 0.0 && b[3] == 0.0
            {
                return px_lerp_float(a, b, 0.5);
            }
            let fa = a[3]*amount;
            let fb = b[3];
            let mut out = px_lerp_float(a, b, fa/(fa+fb));
            out[3] = (fa + fb).clamp(0.0, 1.0);
            out
        },
        
        "Weld" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func_float::<BlendModeNormal>(a, b, amount, modifier, flag);
            out[3] = (a[3]*amount + b[3]).clamp(0.0, 1.0);
            out
        },
        
        "Hard Weld" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func_float::<BlendModeNormal>(a, b, amount, modifier, flag);
            out[3] = out[3].clamp(a[3].min(b[3]), b[3].max(a[3]));
            out
        },
        
        "Clip Weld" => |a, mut b, amount, modifier, flag|
        {
            let al = b[3];
            b[3] = 1.0;
            let mut out = px_func_float::<BlendModeNormal>(a, b, amount, modifier, flag);
            out[3] = al;
            out
        },
        
        _ => px_func_float::<BlendModeNormal>, // Normal, or unknown
    })
}
    
pub (crate) fn find_blend_func(blend_mode : &str) -> IntBlendFn
{
    match blend_mode
    {
        "Composite" => px_func_full::<BlendModeComposite>,
        
        "Multiply" => px_func::<BlendModeMultiply>,
        "Divide" => px_func::<BlendModeDivide>,
        "Screen" => px_func::<BlendModeScreen>,
        "Add" => px_func::<BlendModeAdd>,
        "Glow Add" => px_func::<BlendModeAddGlow>,
        "Subtract" => px_func::<BlendModeSubtract>,
        "Difference" => px_func::<BlendModeDifference>,
        "Signed Diff" => px_func::<BlendModeSignedDifference>,
        "Signed Add" => px_func::<BlendModeSignedAdd>,
        "Negation" => px_func::<BlendModeNegation>,
        "Lighten" => px_func::<BlendModeLighten>,
        "Darken" => px_func::<BlendModeDarken>,
        "Linear Burn" => px_func::<BlendModeLinearBurn>,
        "Color Burn" => px_func::<BlendModeColorBurn>,
        "Color Dodge" => px_func::<BlendModeColorDodge>,
        
        "Glow Dodge" => px_func_full::<BlendModeGlowDodge>,
        
        "Glow" => px_func::<BlendModeGlow>,
        "Reflect" => px_func::<BlendModeReflect>,
        "Overlay" => px_func::<BlendModeOverlay>,
        "Soft Light" => px_func::<BlendModeSoftLight>,
        "Hard Light" => px_func::<BlendModeHardLight>,
        "Vivid Light" => px_func::<BlendModeVividLight>,
        "Linear Light" => px_func::<BlendModeLinearLight>,
        "Pin Light" => px_func::<BlendModePinLight>,
        "Hard Mix" => px_func_extra::<BlendModeHardMix>,
        "Exclusion" => px_func::<BlendModeExclusion>,
        
        "Hue" => px_func_triad::<BlendModeHue>,
        "Saturation" => px_func_triad::<BlendModeSaturation>,
        "Color" => px_func_triad::<BlendModeColor>,
        "Luminosity" => px_func_triad::<BlendModeLuminosity>,
        
        "Flat Hue" => px_func_triad::<BlendModeFlatHue>,
        "Flat Sat" => px_func_triad::<BlendModeFlatSaturation>,
        "Flat Color" => px_func_triad::<BlendModeFlatColor>,
        "Value" => px_func_triad::<BlendModeValue>,
        
        "Hard Sat" => px_func_triad::<BlendModeHardSaturation>,
        "Hard Color" => px_func_triad::<BlendModeHardColor>,
        "Lightness" => px_func_triad::<BlendModeLightness>,
        
        "Erase" => px_func_full::<BlendModeErase>,
        "Reveal" => px_func_full::<BlendModeReveal>,
        "Alpha Mask" => px_func_full::<BlendModeAlphaMask>,
        "Alpha Reject" => px_func_full::<BlendModeAlphaReject>,
        "Under" => px_func_full::<BlendModeUnder>,
        
        "Interpolate" => px_lerp_biased,
        //"Hard Interpolate" => |a, b, amount, _m, _u| px_lerp(b, a, amount * (1.0 - to_float(a[3]))),
        "Hard Interpolate" => |a, b, amount, _m, _u| px_lerp(a, b, amount * (to_float(a[3]))),
        //"Hard Interpolate" => |a, b, amount, _m, _u| px_lerp(b, a, amount * (1.0 - to_float(a[3])) * to_float(b[3])),
        
        "Clamp Erase" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], to_int(to_float(b[3]).min(1.0 - to_float(a[3])))], // used internally
        "Merge Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], to_int(to_float(a[3]) * to_float(b[3]))], // used internally
        "Clip Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], to_int(to_float(a[3]).min(to_float(b[3])))], // used internally
        "Max Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], to_int(to_float(a[3]).max(to_float(b[3])))], // used internally
        "Copy Alpha" => |a, b, _amount, _modifier, _unused| [b[0], b[1], b[2], a[3]], // used internally
        "Copy" => |a, _b, amount, _modifier, _unused| [a[0], a[1], a[2], to_int(to_float(a[3]) * amount)], // used internally
        
        "Dither" => |mut a, b, _amount, _modifier, flag|
        {
            // normal blending, but ignore amount and top alpha (handled by post func)
            a[3] = 255;
            px_func::<BlendModeNormal>(a, b, 1.0, 1.0, flag)
        },
        
        "Weld Under" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func_full::<BlendModeUnder>(a, b, amount, modifier, flag);
            out[3] = to_int((to_float(a[3])*amount + to_float(b[3])).clamp(0.0, 1.0));
            out
        },
        
        "Alpha Antiblend" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func::<BlendModeNormal>([0, 0, 0, a[3]], b, amount, modifier, flag);
            out[3] = to_int((to_float(out[3]) - to_float(a[3]) * amount * modifier).clamp(0.0, 1.0));
            out
        },
        
        "Blend Weld" => |a, b, amount, modifier, _flag|
        {
            if a[3] == 0 && b[3] == 0
            {
                return px_lerp(a, b, 0.5);
            }
            let fa = to_float(a[3]);
            let fa2 = fa*amount*modifier;
            let fb = to_float(b[3]);
            let out_a = (fa2 + fb).clamp(0.0, 1.0);
            let fa3 = fa2 / out_a;
            
            let mut out = b;
            out[0] = to_int((to_float(b[0]) + to_float(a[0]) * fa3).clamp(0.0, 1.0));
            out[1] = to_int((to_float(b[1]) + to_float(a[1]) * fa3).clamp(0.0, 1.0));
            out[2] = to_int((to_float(b[2]) + to_float(a[2]) * fa3).clamp(0.0, 1.0));
            out[3] = to_int(out_a);
            out
        },
        
        "Sum Weld" => |a, b, amount, _modifier, _flag|
        {
            if a[3] == 0 && b[3] == 0
            {
                return px_lerp(a, b, 0.5);
            }
            let fa = to_float(a[3])*amount;
            let fb = to_float(b[3]);
            let mut out = px_lerp(a, b, fa/(fa+fb));
            out[3] = to_int((fa + fb).clamp(0.0, 1.0));
            out
        },
        
        "Weld" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func::<BlendModeNormal>(a, b, amount, modifier, flag);
            out[3] = to_int((to_float(a[3])*amount + to_float(b[3])).clamp(0.0, 1.0));
            out
        },
        
        "Soft Weld" => |mut a, b, amount, _modifier, _flag|
        {
            let fa = to_float(a[3]);
            a[3] = to_int(fa * amount);
            let mut out = px_func::<BlendModeNormal>(a, b, 1.0, 1.0, false);
            
            let fb = to_float(b[3]);
            
            // FIXME this is just a guess and is probably wrong
            let i = (fb + fa * amount).clamp(0.0, 1.0);
            out[3] = to_int(i);
            out
        },
        "Hard Weld" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func::<BlendModeNormal>(a, b, amount, modifier, flag);
            out[3] = out[3].clamp(a[3].min(b[3]), b[3].max(a[3]));
            out
        },
        "Clip Weld" => |a, mut b, amount, modifier, flag|
        {
            let al = b[3];
            b[3] = 255;
            let mut out = px_func::<BlendModeNormal>(a, b, amount, modifier, flag);
            out[3] = al;
            out
        },
        /*
        "Merge Weld" => |a, b, amount, modifier, flag|
        {
            let mut out = px_func::<BlendModeNormal>(a, b, amount, modifier, flag);
            let a = unlerp(out[3], a[3], b[3]).clamp(0.0, 1.0);
            let mut out = px_func::<BlendModeNormal>(a, b, amount, 1.0, false);
            out
        },
        */
        
        _ => px_func::<BlendModeNormal>, // Normal, or unknown
    }
}

fn dither<T : Sized>(blended : T, base : T, mut amount : f32, coord : [usize; 2]) -> T
{
    let x = coord[0];
    let y = coord[1];
    amount = 1.0 - amount;
    amount += ((x  +y  +1)&1) as f32 * (1.0/2.0);
    amount += ((    y  +1)&1) as f32 * (1.0/4.0);
    amount += ((x/2+y/2+1)&1) as f32 * (1.0/8.0);
    amount += ((    y/2+1)&1) as f32 * (1.0/16.0);
    amount += ((x/4+y/4+1)&1) as f32 * (1.0/32.0);
    amount += ((    y/4+1)&1) as f32 * (1.0/64.0);
    if amount >= 1.0
    {
        base
    }
    else
    {
        blended
    }
}

type FloatPostFn = fn([f32; 4], [f32; 4], [f32; 4], f32, f32, bool, [usize; 2]) -> [f32; 4];
type IntPostFn = fn([u8; 4], [u8; 4], [u8; 4], f32, f32, bool, [usize; 2]) -> [u8; 4];

pub (crate) fn find_post_func_float(blend_mode : &str) -> FloatPostFn
{
    match blend_mode
    {
        "Dither" => |blended, top, base, mut amount, _modifier, _flag, coord|
        {
            // blend original top alpha into amount because we threw it out in the blending stage
            amount *= top[3];
            dither::<[f32; 4]>(blended, base, amount, coord)
        },
        _ => |blended, _top, _base, _amount, _modifier, _coord, _flag| blended,
    }
}
pub (crate) fn find_post_func(blend_mode : &str) -> IntPostFn
{
    match blend_mode
    {
        "Dither" => |blended, top, base, mut amount, _modifier, _flag, coord|
        {
            // blend original top alpha into amount because we threw it out in the blending stage
            amount *= to_float(top[3]);
            dither::<[u8; 4]>(blended, base, amount, coord)
        },
        _ => |blended, _top, _base, _amount, _modifier, _coord, _flag| blended,
    }
}

#[inline]
pub (crate) fn to_float(x : u8) -> f32
{
    //(x as f32)/255.0
    (x as f32)*(1.0/255.0)
}
#[inline]
pub (crate) fn to_int(x : f32) -> u8
{
    (x.clamp(0.0, 1.0)*255.0 + 0.5) as u8 // correct rounding is too slow
}
#[inline]
pub (crate) fn px_to_float<const N : usize>(x : [u8; N]) -> [f32; N]
{
    let mut ret = [0.0; N];
    for i in 0..N
    {
        ret[i] = to_float(x[i]);
    }
    ret
}
#[inline]
pub (crate) fn px_to_int<const N : usize>(x : [f32; N]) -> [u8; N]
{
    let mut ret = [0; N];
    for i in 0..N
    {
        ret[i] = to_int(x[i]);
    }
    ret
}

#[inline]
pub (crate) fn rgb_to_hsv(rgba : [f32; 4]) -> [f32; 4]
{
    let v = rgba[0].max(rgba[1]).max(rgba[2]);
    let c = v - rgba[0].min(rgba[1]).min(rgba[2]);
    let s = if v > 0.0 { c / v } else { 0.0 };
    let h;
    if c == 0.0
    {
        h = 0.0;
    }
    else if v == rgba[0]
    {
        h = 60.0 * (rgba[1] - rgba[2])/c;
    }
    else if v == rgba[1]
    {
        h = 60.0 * (rgba[2] - rgba[0])/c + 120.0;
    }
    else
    {
        h = 60.0 * (rgba[0] - rgba[1])/c + 240.0;
    }
    [h, s, v, rgba[3]]
}
#[inline]
pub (crate) fn hsv_to_rgb(hsva : [f32; 4]) -> [f32; 4]
{
    let c = hsva[2] * hsva[1];
    let h2 = hsva[0] / 60.0;
    let x = c * (1.0 - ((h2%2.0) - 1.0).abs());
    
    let triad = [
        [c, x, 0.0],
        [x, c, 0.0],
        [0.0, c, x],
        [0.0, x, c],
        [x, 0.0, c],
        [c, 0.0, x],
    ][h2.floor() as usize % 6];
    
    let m = hsva[2] - c;
    [triad[0] + m, triad[1] + m, triad[2] + m, hsva[3]]
}


#[inline]
pub (crate) fn rgb_to_hsl(rgba : [f32; 4]) -> [f32; 4]
{
    let hsva = rgb_to_hsv(rgba);
    let mut hsla : [f32; 4] = [0.0, 0.0, 0.0, 0.0];
    hsla[0] = hsva[0];
    hsla[2] = hsva[2] * (1.0 - hsva[1]/2.0);
    if hsla[2] == 0.0 || 1.0 - hsla[2] == 0.0
    {
        hsla[1] = 0.0;
    }
    else if hsla[2] < 0.5
    {
        hsla[1] = hsva[1] * hsva[2] / (2.0 * hsla[2]);
    }
    else
    {
        hsla[1] = hsva[1] * hsva[2] / (2.0 - (2.0 * hsla[2]));
    }
    hsla[3] = hsva[3];
    
    hsla
}
#[inline]
pub (crate) fn hsl_to_rgb(hsla : [f32; 4]) -> [f32; 4]
{
    let mut hsva : [f32; 4] = [0.0, 0.0, 0.0, 0.0];
    hsva[0] = hsla[0];
    hsva[2] = hsla[2] + hsla[1] * hsla[2].min(1.0 - hsla[2]);
    if hsva[2] == 0.0
    {
        hsva[1] = 0.0;
    }
    else
    {
        hsva[1] = 2.0 * (1.0 - (hsla[2] / hsva[2]));
    }
    hsva[3] = hsla[3];
    
    hsv_to_rgb(hsva)
}
