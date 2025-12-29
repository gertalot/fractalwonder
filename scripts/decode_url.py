#!/usr/bin/env python3
"""
Decode FractalWonder URL hash to viewport coordinates.

Usage:
    python decode_url.py <url_or_hash>
    python decode_url.py "http://127.0.0.1:8080/fractalwonder/#v1:7ZvN..."
    python decode_url.py "v1:7ZvN..."

Options:
    --raw   Include raw JSON output
    --json  Output only the decoded JSON (machine-readable)
"""

import base64
import json
import sys
import zlib
from decimal import Decimal, getcontext
from urllib.parse import urlparse


def decode_fractalwonder_url(url_or_hash: str) -> dict:
    """Decode a FractalWonder URL or hash to its JSON state."""
    # Extract hash from full URL if needed
    if url_or_hash.startswith(("http://", "https://")):
        parsed = urlparse(url_or_hash)
        hash_data = parsed.fragment
    else:
        hash_data = url_or_hash

    # Strip v1: prefix
    if not hash_data.startswith("v1:"):
        raise ValueError("Invalid format: expected 'v1:' prefix")

    encoded = hash_data[3:]  # Remove "v1:"

    # Base64 decode (URL-safe, no padding)
    # Add padding if needed
    padding = 4 - (len(encoded) % 4)
    if padding != 4:
        encoded += "=" * padding

    compressed = base64.urlsafe_b64decode(encoded)

    # Decompress (DEFLATE, not gzip - use raw deflate with -15 window bits)
    decompressed = zlib.decompress(compressed, -zlib.MAX_WBITS)

    # Parse JSON
    return json.loads(decompressed.decode("utf-8"))


def binary_to_decimal(binary_str: str) -> Decimal:
    """Convert a binary string (e.g., '0.1010') to a Decimal value."""
    # Set high precision for extreme zoom depths
    getcontext().prec = 1000

    if not binary_str:
        return Decimal(0)

    # Handle sign
    negative = binary_str.startswith("-")
    if negative:
        binary_str = binary_str[1:]

    # Split into integer and fractional parts
    if "." in binary_str:
        int_part, frac_part = binary_str.split(".", 1)
    else:
        int_part = binary_str
        frac_part = ""

    # Convert integer part
    result = Decimal(int(int_part, 2)) if int_part else Decimal(0)

    # Convert fractional part
    if frac_part:
        frac_value = Decimal(0)
        for i, bit in enumerate(frac_part, 1):
            if bit == "1":
                frac_value += Decimal(2) ** (-i)
        result += frac_value

    return -result if negative else result


def format_bigfloat(bf: dict) -> str:
    """Format a BigFloat value as a decimal string."""
    if isinstance(bf, dict) and "value" in bf:
        binary_value = bf["value"]
        decimal_value = binary_to_decimal(binary_value)
        # Format with scientific notation if very small
        if abs(decimal_value) < Decimal("1e-10") and decimal_value != 0:
            return f"{decimal_value:.50E}"
        return str(decimal_value)[:80]  # Truncate for display
    return str(bf)


def format_viewport(state: dict, show_raw: bool = False) -> str:
    """Format the viewport data for display."""
    lines = []

    lines.append("=" * 60)
    lines.append("FRACTALWONDER STATE")
    lines.append("=" * 60)

    # Viewport
    viewport = state.get("viewport", {})
    center = viewport.get("center", [None, None])

    lines.append("\nVIEWPORT:")
    lines.append(f"  Center X:  {format_bigfloat(center[0])}")
    lines.append(f"  Center Y:  {format_bigfloat(center[1])}")
    lines.append(f"  Width:     {format_bigfloat(viewport.get('width'))}")
    lines.append(f"  Height:    {format_bigfloat(viewport.get('height'))}")

    # Calculate zoom depth from BigFloat binary representation
    try:
        width_data = viewport.get("width", {})
        if isinstance(width_data, dict) and "value" in width_data:
            binary_value = width_data["value"]
            # Count leading zeros after "0." to estimate zoom depth
            if "." in binary_value:
                frac_part = binary_value.split(".")[1]
                leading_zeros = len(frac_part) - len(frac_part.lstrip("0"))
                # Convert binary exponent to decimal: 2^n ≈ 10^(n*log10(2)) ≈ 10^(n*0.301)
                zoom_depth = int(leading_zeros * 0.301)
                lines.append(f"  Zoom Depth: ~10^{zoom_depth} (2^{leading_zeros})")
                lines.append(f"  Precision:  {width_data.get('precision_bits', 'N/A')} bits")
    except (ValueError, IndexError, TypeError):
        pass

    # Config
    lines.append(f"\nCONFIG:")
    lines.append(f"  Config ID:    {state.get('config_id', 'unknown')}")
    lines.append(f"  Palette:      {state.get('palette_name', 'unknown')}")
    lines.append(f"  Version:      {state.get('version', 'unknown')}")

    # Render settings
    render = state.get("render_settings", {})
    if render:
        lines.append(f"\nRENDER SETTINGS:")
        lines.append(f"  Cycle Count:  {render.get('cycle_count', 'N/A')}")
        lines.append(f"  Use GPU:      {render.get('use_gpu', 'N/A')}")
        lines.append(f"  X-Ray:        {render.get('xray_enabled', 'N/A')}")

    if show_raw:
        lines.append("\n" + "=" * 60)
        lines.append("RAW JSON:")
        lines.append("=" * 60)
        lines.append(json.dumps(state, indent=2))

    return "\n".join(lines)


def main():
    args = sys.argv[1:]

    if not args or "-h" in args or "--help" in args:
        print(__doc__)
        sys.exit(0 if "-h" in args or "--help" in args else 1)

    show_raw = "--raw" in args
    json_only = "--json" in args

    # Filter out flags to get the URL
    url_or_hash = next((a for a in args if not a.startswith("-")), None)

    if not url_or_hash:
        print("Error: No URL provided", file=sys.stderr)
        sys.exit(1)

    try:
        state = decode_fractalwonder_url(url_or_hash)
        if json_only:
            print(json.dumps(state, indent=2))
        else:
            print(format_viewport(state, show_raw=show_raw))
    except Exception as e:
        print(f"Error decoding URL: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
