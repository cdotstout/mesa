#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright 2026 The Magma GPU Project

import sys

import argparse
import os
import typing as T

from pathlib import Path

from gorgonzola.api_generator import ApiGenerator
from gorgonzola.common import OutputType

GORGONZOLA_DESC_STRING = 'gorgonzola: generate Rust APIs from toml description'
GORGONZOLA_API_DIR_STRING = 'Generate code from custom api directory'
GORGONZOLA_OUT_DIR_INFO_STRING = 'Where to place the output api definitions'
GORGONZOLA_OUTPUT_TYPE = 'The type of output (client, server) '


def run(args: T.List[str]) -> int:
    parser = argparse.ArgumentParser(description=GORGONZOLA_DESC_STRING)
    parser.add_argument('--api-dir', required=True, help=GORGONZOLA_API_DIR_STRING)
    parser.add_argument('--out-dir', required=True, help=GORGONZOLA_OUT_DIR_INFO_STRING)
    parser.add_argument(
        '--output-type',
        type=OutputType,
        choices=list(OutputType),
        default=OutputType.CLIENT,
        help=GORGONZOLA_OUTPUT_TYPE,
    )
    options = parser.parse_args(args)

    api_dir = Path(os.path.abspath(options.api_dir))
    out_dir = Path(os.path.abspath(options.out_dir))

    api_generator = ApiGenerator(api_dir, out_dir, output_type=options.output_type)
    api_generator.parse()
    api_generator.emit()
    return 0


def main() -> int:
    return run(sys.argv[1:])


if __name__ == '__main__':
    sys.exit(main())
