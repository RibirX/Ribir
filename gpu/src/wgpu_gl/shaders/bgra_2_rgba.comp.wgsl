
struct PrimeIndices {
  indices: array<u32>;
};
[[group(0), binding(0)]]
var<storage,read_write> prime_indices: PrimeIndices;

[[stage(compute), workgroup_size(1, 1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
  let index: u32 = global_id.x;
  let bgra: u32 = prime_indices.indices[index];

  let rgba = ((bgra << 16u) & 0x00FF0000u) |
      ((bgra >> 16u) & 0x000000FFu) |
      (bgra & 0xFF00FF00u);
  prime_indices.indices[index] = rgba;
}
