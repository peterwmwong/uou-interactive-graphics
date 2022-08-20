#pragma once

// TODO: Use a more compact format for storing normals
// See https://aras-p.info/texts/CompactNormalStorage.html#method04spheremap
struct TriNormalsIndex {
    packed_half3   normals[3];
    unsigned short index;
    // TODO: START HERE
    // TODO: START HERE
    // TODO: START HERE
    // Add helper method for creating a normal (half3) from a barycentric coordinate (xy).
    // - Extract code from x-rt shader
    // - Reuse in proj-6-ray-traced-reflections
};

