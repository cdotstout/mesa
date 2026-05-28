#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright 2026 The Magma GPU Project

import sys
from enum import Enum

import argparse
import os
import typing as T

from pathlib import Path

from gorgonzola.api_generator import ApiGenerator
from gorgonzola.common import OutputType

GORGONZOLA_DESC_STRING = 'gorgonzola: generate Rust APIs from toml description'
GORGONZOLA_API_INFO_STRING = 'Generate code from chosen reference API'
GORGONZOLA_API_DIR_STRING = 'Generate code from from custom api directory'
GORGONZOLA_OUT_DIR_INFO_STRING = 'Where to place the output api definitions'
GORGONZOLA_OUTPUT_TYPE = 'The type of output (client, server) '


def run(directory: Path, args: T.List[str]) -> int:
    parser = argparse.ArgumentParser(description=GORGONZOLA_DESC_STRING)
    parser.add_argument('api', nargs='?', default=None, help=GORGONZOLA_API_INFO_STRING)
    parser.add_argument('--api-dir', help=GORGONZOLA_API_DIR_STRING)
    parser.add_argument('--out-dir', help=GORGONZOLA_OUT_DIR_INFO_STRING)
    parser.add_argument(
        '--output-type',
        type=OutputType,
        choices=list(OutputType),
        default=OutputType.CLIENT,
        help=GORGONZOLA_OUTPUT_TYPE,
    )
    options = parser.parse_args(args)

    if options.api_dir:
        api_dir = Path(os.path.abspath(options.api_dir))
    else:
        print(f'Error: Please specify --api_dir')
        return 1

    if options.out_dir:
        out_dir = Path(os.path.abspath(options.out_dir))
    else:
        print(f'Error: Please specify an output directory via --out_dir')
        return 1

    api_generator = ApiGenerator(api_dir, out_dir, output_type=options.output_type)
    if not api_generator.parse():
        print(f'Error: Failed to parse API from {api_dir}')
        return 1

    api_generator.emit()
    return 0


def main() -> int:
    path = os.path.abspath(sys.argv[0])
    directory = Path(path).parent
    return run(directory, sys.argv[1:])


if __name__ == '__main__':
    sys.exit(main())
