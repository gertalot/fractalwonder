//! GPU BLA integration tests.

#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use fractalwonder_compute::BlaTable;
    use fractalwonder_compute::ReferenceOrbit;
    use fractalwonder_core::{BigFloat, HDRFloat};
    use fractalwonder_gpu::GpuBlaEntry;

    #[test]
    fn bla_table_serializes_to_gpu_format() {
        // Create a simple reference orbit
        let c_ref = (BigFloat::with_precision(-0.5, 128), BigFloat::zero(128));
        let orbit = ReferenceOrbit::compute(&c_ref, 100);

        // Compute BLA table
        let dc_max = HDRFloat::from_f64(1e-10);
        let bla_table = BlaTable::compute(&orbit, &dc_max);

        // Convert to GPU format
        let gpu_entries: Vec<GpuBlaEntry> = bla_table
            .entries
            .iter()
            .map(GpuBlaEntry::from_bla_entry)
            .collect();

        // Verify we got entries
        assert!(!gpu_entries.is_empty());
        assert_eq!(gpu_entries.len(), bla_table.entries.len());

        // Verify first entry matches
        let first_cpu = &bla_table.entries[0];
        let first_gpu = &gpu_entries[0];
        assert_eq!(first_gpu.l, first_cpu.l);
        assert_eq!(first_gpu.a_re_head, first_cpu.a.re.head);
    }
}
