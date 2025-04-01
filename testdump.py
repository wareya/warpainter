import sys
from inspect import ismethod
from psd_tools import PSDImage
from psd_tools.api.layers import AdjustmentLayer

def print_object_properties(obj):
    s = ""
    for attr in dir(obj):
        if attr.startswith("_"): continue
        try:
            if ismethod(getattr(obj, attr)): continue
            s += f"{attr} : {getattr(obj, attr)}; "
        except Exception as e:
            s += f"{attr} : <not accessible>; "
    return s

def dump_layer(layer, depth=0):
    """Recursively dump layer information, including adjustment layers."""
    indent = '  ' * depth
    print(f"{indent}  vvvvv {print_object_properties(layer)}")
    print(f"{indent}- Layer: {layer.name}")
    print(f"{indent}  Kind: {layer.kind}")
    print(f"{indent}  Visible: {layer.visible}")
    print(f"{indent}  Opacity: {layer.opacity}")
    print(f"{indent}  Blend Mode: {layer.blend_mode}")
    print(f"{indent}  Bbox: {layer.bbox}")

    if isinstance(layer, AdjustmentLayer):
        print(f"{indent}  (Adjustment Layer) {layer.kind}")
        if layer.kind == 'brightnesscontrast':
            print(f"{indent}    Brightness: {layer.brightness}")
            print(f"{indent}    Contrast: {layer.contrast}")
        elif layer.kind == 'curves':
            print(f"{indent}    Curves: {layer.data}")
        elif layer.kind == 'levels':
            print(f"{indent}    Levels: {layer.data}")
        elif layer.kind == 'huesaturation':
            print(f"{indent}    Data: {layer.data}")
        elif layer.kind == 'colorbalance':
            print(f"{indent}    Shadows: {layer.shadows}")
            print(f"{indent}    Midtones: {layer.midtones}")
            print(f"{indent}    Highlights: {layer.highlights}")

    if layer.has_mask():
        mask = layer.mask
        print(f"{indent}  Mask: Present")
        print(f"{indent}    Background Color: {mask.background_color}")
        print(f"{indent}    Flags: {print_object_properties(mask.flags)}")
        print(f"{indent}    Bbox: {mask.bbox}")
        print(f"{indent}    Disabled: {mask.disabled}")
    else:
        print(f"{indent}  Mask: None")

    if layer.is_group():
        print(f"{indent}  (Layer Group)")
        for sublayer in layer:
            dump_layer(sublayer, depth + 1)

def dump_psd_metadata(psd_path):
    psd = PSDImage.open(psd_path)
    print(f"PSD File: {psd_path}")
    print(f"Dimensions: {psd.width}x{psd.height}")
    print(f"Color Mode: {psd.color_mode}")
    print(f"Number of Layers: {len(psd)}")
    print(f"Metadata: {psd.image_resources}")

    print("\nLayers:")
    for layer in psd:
        dump_layer(layer)
        
    # Check and display the global layer mask information if available
    if hasattr(psd, 'layer_and_mask') and psd.layer_and_mask.global_layer_mask_info:
        mask_info = psd.layer_and_mask_info.global_layer_mask_info
        print("\nGlobal Layer Mask Info:")
        print(f"  Overlay Color: {mask_info.overlay_color}")
        print(f"  Opacity: {mask_info.opacity}")
        print(f"  Kind: {mask_info.kind}")
    else:
        print("no global mask info");


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python dump_psd_metadata.py <psd_file>")
        sys.exit(1)

    psd_path = sys.argv[1]
    dump_psd_metadata(psd_path)