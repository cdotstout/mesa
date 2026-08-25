/*
 * Copyright 2026 The Magma GPU Project
 * SPDX-License-Identifier: MIT
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>

struct intel_device_info;
struct magma_physical_device;
typedef struct magma_physical_device *magma_physical_device_t;

bool
intel_device_info_magma_get_info(magma_physical_device_t pdev,
                                 struct intel_device_info *devinfo,
                                 int min_ver, int max_ver);
