#!/bin/sh
glslangValidator -V ./shaders/geometry.vert -o ./shaders/geometry.vert.spv
glslangValidator -V ./shaders/geometry.frag -o ./shaders/geometry.frag.spv

