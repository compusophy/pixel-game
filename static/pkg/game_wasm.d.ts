/* tslint:disable */
/* eslint-disable */

export class PixelBuffer {
    free(): void;
    [Symbol.dispose](): void;
    static create_join_msg(name: string): Uint8Array;
    height(): number;
    constructor(w: number, h: number);
    on_click(screen_x: number, screen_y: number): void;
    on_scroll(delta: number, cursor_x: number, cursor_y: number): void;
    pointer(): number;
    poll_message(): Uint8Array | undefined;
    receive_message(data: Uint8Array): void;
    resize(w: number, h: number): void;
    set_zoom(z: number): void;
    tick(time: number): void;
    width(): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_pixelbuffer_free: (a: number, b: number) => void;
    readonly pixelbuffer_new: (a: number, b: number) => number;
    readonly pixelbuffer_resize: (a: number, b: number, c: number) => void;
    readonly pixelbuffer_pointer: (a: number) => number;
    readonly pixelbuffer_width: (a: number) => number;
    readonly pixelbuffer_height: (a: number) => number;
    readonly pixelbuffer_tick: (a: number, b: number) => void;
    readonly pixelbuffer_on_click: (a: number, b: number, c: number) => void;
    readonly pixelbuffer_set_zoom: (a: number, b: number) => void;
    readonly pixelbuffer_on_scroll: (a: number, b: number, c: number, d: number) => void;
    readonly pixelbuffer_receive_message: (a: number, b: number, c: number) => void;
    readonly pixelbuffer_poll_message: (a: number) => [number, number];
    readonly pixelbuffer_create_join_msg: (a: number, b: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
