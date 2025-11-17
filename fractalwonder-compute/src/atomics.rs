use wasm_bindgen::prelude::*;

/// Bindings to JavaScript Atomics API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Atomics, js_name = add)]
    fn atomics_add(typedArray: &js_sys::Int32Array, index: u32, value: i32) -> i32;

    #[wasm_bindgen(js_namespace = Atomics, js_name = load)]
    fn atomics_load(typedArray: &js_sys::Int32Array, index: u32) -> i32;

    #[wasm_bindgen(js_namespace = Atomics, js_name = store)]
    fn atomics_store(typedArray: &js_sys::Int32Array, index: u32, value: i32) -> i32;
}

/// Atomically fetch and add to u32 value in buffer
pub fn atomic_fetch_add_u32(buffer: &js_sys::ArrayBuffer, byte_offset: u32, value: u32) -> u32 {
    debug_assert_eq!(byte_offset % 4, 0, "byte_offset must be 4-byte aligned");
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_add(&int32_array, index, value as i32) as u32
}

/// Atomically load u32 value from buffer
pub fn atomic_load_u32(buffer: &js_sys::ArrayBuffer, byte_offset: u32) -> u32 {
    debug_assert_eq!(byte_offset % 4, 0, "byte_offset must be 4-byte aligned");
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_load(&int32_array, index) as u32
}

/// Atomically store u32 value to buffer
pub fn atomic_store_u32(buffer: &js_sys::ArrayBuffer, byte_offset: u32, value: u32) -> u32 {
    debug_assert_eq!(byte_offset % 4, 0, "byte_offset must be 4-byte aligned");
    let int32_array = js_sys::Int32Array::new(buffer);
    let index = byte_offset / 4; // Convert byte offset to i32 index
    atomics_store(&int32_array, index, value as i32) as u32
}
