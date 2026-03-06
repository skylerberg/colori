#!/usr/bin/env python3
"""Export a training checkpoint (.pt) to ONNX format."""

import argparse
import sys
import os

import torch

# Add training directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from model import ColoriNet
from run_training import export_onnx


def main():
    parser = argparse.ArgumentParser(description="Export checkpoint to ONNX")
    parser.add_argument("checkpoint", help="Path to .pt checkpoint file")
    parser.add_argument("-o", "--output", default=None, help="Output .onnx path")
    args = parser.parse_args()

    if args.output is None:
        base = os.path.splitext(args.checkpoint)[0]
        args.output = base + ".onnx"

    if torch.cuda.is_available():
        device = torch.device("cuda")
    elif torch.backends.mps.is_available():
        device = torch.device("mps")
    else:
        device = torch.device("cpu")

    model = ColoriNet().to(device)
    checkpoint = torch.load(args.checkpoint, map_location=device, weights_only=False)
    model.load_state_dict(checkpoint["model"])
    print(f"Loaded checkpoint: {args.checkpoint}")

    export_onnx(model, device, args.output)


if __name__ == "__main__":
    main()
