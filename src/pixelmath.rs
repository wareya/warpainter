#[inline]
fn lerp(a : f32, b : f32, t : f32) -> f32
{
    a * (1.0 - t) + b * t
}

#[inline]
// return what the output alpha should be given the two input alpha values
fn alpha_combine(a : f32, b : f32) -> f32
{
    b * (1.0 - a) + a
}

#[inline]
pub (crate) fn px_lerp_float(a : [f32; 4], b : [f32; 4], amount : f32) -> [f32; 4]
{
    let mut r = [0.0; 4];
    for i in 0..4
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
pub (crate) fn px_lerp_biased_float(a : [f32; 4], b : [f32; 4], amount : f32, _modifier : f32) -> [f32; 4]
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
pub (crate) fn px_lerp_biased(a : [u8; 4], b : [u8; 4], amount : f32, _modifier : f32) -> [u8; 4]
{
    px_to_int(px_lerp_biased_float(px_to_float(a), px_to_float(b), amount, _modifier))
}


pub (crate) struct BlendModeHardMix;
impl BlendModeSimpleExtra for BlendModeHardMix
{
    fn blend(mut top : f32, bottom : f32, _opacity : f32, mut fill : f32) -> f32
    {
        //(((top + bottom - 1.0 - 0.5/255.0) * 2.0 * 255.0_f32.powf(fill*2.0)/255.0) + 0.5).clamp(0.0, 1.0)
        //(((top + bottom - 1.0 - 0.5/255.0) * (fill*2.0).powf(7.0) * 2.0) + 0.5).clamp(0.0, 1.0)
        //top = lerp(bottom, top, fill*0.5+0.5);
        top = lerp(bottom, top, (fill * (1.0/0.75)).clamp(0.0, 1.0));
        //top = lerp(bottom, top, (fill).clamp(0.0, 1.0));
        fill *= 2.0;
        let mut n = (top + bottom - 1.0 - 0.5/255.0) * 2.0;
        n = if fill > 1.0
        {
            //let mut f = 1.0 / ((2.0 - fill) * (0.97));
            let mut f = 1.0 / (2.0 - fill);
            (n * (f * 0.75)) + 0.5
        }
        else
        {
            (n * (fill * 0.75)) + 0.5
        };
        n = lerp(bottom, n, (fill).clamp(0.0, 1.0));
        n.clamp(0.0, 1.0)
    }
}
#[inline]
pub (crate) fn px_func_extra_float<T : BlendModeSimpleExtra>
    (mut a : [f32; 4], b : [f32; 4], amount : f32, mut modifier : f32)
    -> [f32; 4]
{
    if false
    {
        modifier *= a[3];
        a[3] = amount;
    }
    else
    {
        a[3] *= amount;
    }
    
    /*
    if a[3] * modifier == 0.0
    {
        return b;
    }
    else if b[3] == 0.0
    {
        let mut a = a;
        a[3] *= modifier;
        return a;
    }
    */

    let mut r = [0.0; 4];
    
    // a is top layer, b is bottom
    let b_under_a = b[3] * (1.0 - a[3] * modifier);
    r[3] = a[3] * modifier + b_under_a;
    
    let m = 1.0 / r[3];
    let a_a = a[3] * m;
    let b_a = b_under_a * m;
    
    //if a[3] != 1.0
    {
        for i in 0..3
        {
          //r[i] = lerp(a[i], T::blend(a[i], b[i], amount, modifier), b[3]) * a_a + b[i] * b_a;
            //r[i] = lerp(a[i], T::blend(a[i], b[i], amount, modifier), 1.0 - b_under_a) * a_a + b[i] * b_a;
            r[i] = lerp(a[i], T::blend(a[i], b[i], amount, modifier), b[3]);
            //r[i] = lerp(r[i], b[i], 1.0 - a[3] * b[3]);
            r[i] = lerp(r[i], b[i], 1.0 - a_a * (modifier*2.0).clamp(0.0, 1.0));
            //r[i] = lerp(r[i], b[i], b_a * (modifier*2.0).clamp(0.0, 1.0));
            //r[i] = lerp(r[i], b[i], 1.0 - (1.0 - b_a) * (modifier*4.0).clamp(0.0, 1.0));
            //r[i] = lerp(b[i], r[i], a[3] * (1.0 - ((1.0 - b[3]) * a[3])));
            //r[i] = lerp(r[i], b[i], lerp(b_under_a, 1.0 - a[3], modifier));
            //r[i] = lerp(r[i], b[i], 1.0 - a[3] * (modifier*2.0).clamp(0.0, 1.0));
            //r[i] = lerp(r[i], b[i], b[3] * (1.0 - a[3]));
            //r[i] = lerp(b[i], r[i], a[3] * modifier * (1.0 - b[3]));
            //r[i] = lerp(r[i], b[i], b_under_a);
        }
    }
    
    r
}

#[inline]
pub (crate) fn px_func_extra<T : BlendModeSimpleExtra>
    (a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32)
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
    px_to_int(px_func_extra_float::<T>(px_to_float(a), px_to_float(b), amount, modifier))
}



#[inline]
pub (crate) fn px_func_float<T : BlendModeSimple>
    (mut a : [f32; 4], b : [f32; 4], amount : f32, modifier : f32)
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
    r[3] = a[3] + b_under_a;
    let m = 1.0 / r[3];
    
    let a_a = a[3] * m;
    let b_a = b_under_a * m;
    
    for i in 0..3
    {
        r[i] = lerp(a[i], T::blend(a[i], b[i]), b[3]) * a_a + b[i] * b_a;
    }
    
    r
}
#[inline]
pub (crate) fn px_func<T : BlendModeSimple>
    (a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32)
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
    px_to_int(px_func_float::<T>(px_to_float(a), px_to_float(b), amount, modifier))
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
            top - (1.0 - 2.0 * bottom) * top * (1.0 - top)
        }
        else
        {
            top + (2.0 * bottom - 1.0) * (top.sqrt() - top)
        }
    }
}
pub (crate) struct BlendModeHardLight;
impl BlendModeSimple for BlendModeHardLight
{
    fn blend(top : f32, bottom : f32) -> f32
    {
        if top > 0.5
        {
            let t = top * 2.0 - 1.0;
            1.0 - (1.0 - t) * (1.0 - bottom)
        }
        else
        {
            top * 2.0 * bottom
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
pub (crate) fn px_func_triad_float<T : BlendModeTriad>(mut a : [f32; 4], b : [f32; 4], amount : f32, _modifier : f32) -> [f32; 4]
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
pub (crate) fn px_func_triad<T : BlendModeTriad>(a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32) -> [u8; 4]
{
    px_to_int(px_func_triad_float::<T>(px_to_float(a), px_to_float(b), amount, modifier))
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
pub (crate) fn px_func_full_float<T : BlendModeFull>(a : [f32; 4], b : [f32; 4], amount : f32, _modifier : f32) -> [f32; 4]
{
    T::blend(a, b, amount)
}
#[inline]
pub (crate) fn px_func_full<T : BlendModeFull>(a : [u8; 4], b : [u8; 4], amount : f32, modifier : f32) -> [u8; 4]
{
    px_to_int(px_func_full_float::<T>(px_to_float(a), px_to_float(b), amount, modifier))
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

type FloatBlendFn = dyn Fn([f32; 4], [f32; 4], f32, f32) -> [f32; 4];
type IntBlendFn = fn([u8; 4], [u8; 4], f32, f32) -> [u8; 4];

pub (crate) fn find_blend_func_float(blend_mode : &str) -> Box<FloatBlendFn>
{
    Box::new(match blend_mode
    {
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
        
        "Interpolate" => px_lerp_biased_float,
        
        "Clip Alpha" => |a, b, _amount, _modifier| [b[0], b[1], b[2], a[3].min(b[3])], // used internally
        "Copy Alpha" => |a, b, _amount, _modifier| [b[0], b[1], b[2], a[3]], // used internally
        "Copy" => |a, _b, amount, _modifier| [a[0], a[1], a[2], a[3] * amount], // used internally
        
        "Dither" => |mut a, b, _amount, _modifier|
        {
            // normal blending, but ignore amount and top alpha (handled by post func)
            a[3] = 1.0;
            px_func_float::<BlendModeNormal>(a, b, 1.0, 1.0)
        },
        
        "Weld" => |a, b, amount, modifier|
        {
            let mut out = px_func_float::<BlendModeNormal>(a, b, amount, modifier);
            out[3] = (a[3] + b[3]*amount).clamp(0.0, 1.0);
            out
        },
        
        _ => px_func_float::<BlendModeNormal>, // Normal, or unknown
    })
}
    
pub (crate) fn find_blend_func(blend_mode : &str) -> IntBlendFn
{
    match blend_mode
    {
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
        
        "Interpolate" => px_lerp_biased,
        
        "Clip Alpha" => |a, b, _amount, _modifier| [b[0], b[1], b[2], to_int(to_float(a[3]).min(to_float(b[3])))], // used internally
        "Copy Alpha" => |a, b, _amount, _modifier| [b[0], b[1], b[2], a[3]], // used internally
        "Copy" => |a, _b, amount, _modifier| [a[0], a[1], a[2], to_int(to_float(a[3]) * amount)], // used internally
        
        "Dither" => |mut a, b, _amount, _modifier|
        {
            // normal blending, but ignore amount and top alpha (handled by post func)
            a[3] = 255;
            px_func::<BlendModeNormal>(a, b, 1.0, 1.0)
        },
        
        "Weld" => |a, b, amount, modifier|
        {
            let mut out = px_func::<BlendModeNormal>(a, b, amount, modifier);
            out[3] = to_int(to_float(a[3]) + to_float(b[3])*amount);
            out
        },
        
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

type FloatPostFn = fn([f32; 4], [f32; 4], [f32; 4], f32, f32, [usize; 2]) -> [f32; 4];
type IntPostFn = fn([u8; 4], [u8; 4], [u8; 4], f32, f32, [usize; 2]) -> [u8; 4];

pub (crate) fn find_post_func_float(blend_mode : &str) -> FloatPostFn
{
    match blend_mode
    {
        "Dither" => |blended, top, base, mut amount, _modifier, coord|
        {
            // blend original top alpha into amount because we threw it out in the blending stage
            amount *= top[3];
            dither::<[f32; 4]>(blended, base, amount, coord)
        },
        _ => |blended, _top, _base, _amount, _modifier, _coord| blended,
    }
}
pub (crate) fn find_post_func(blend_mode : &str) -> IntPostFn
{
    match blend_mode
    {
        "Dither" => |blended, top, base, mut amount, _modifier, coord|
        {
            // blend original top alpha into amount because we threw it out in the blending stage
            amount *= to_float(top[3]);
            dither::<[u8; 4]>(blended, base, amount, coord)
        },
        _ => |blended, _top, _base, _amount, _modifier, _coord| blended,
    }
}

#[inline]
pub (crate) fn to_float(x : u8) -> f32
{
    (x as f32)/255.0
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
