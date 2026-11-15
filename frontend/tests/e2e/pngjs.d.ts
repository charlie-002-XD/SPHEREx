// Minimal ambient typing for pngjs. @types/pngjs is deliberately not
// installed: it depends on @types/node, and pulling node typings into
// node_modules would turn pre-existing `@ts-expect-error` directives in
// tests/ (written for an environment without node types) into unused
// directives and break `tsc --noEmit`.
declare module "pngjs" {
  export interface PngImage {
    width: number;
    height: number;
    data: Uint8Array;
  }
  export const PNG: {
    sync: {
      read(buffer: Uint8Array): PngImage;
    };
  };
}
