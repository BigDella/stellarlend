declare module 'cls-hooked' {
  interface Namespace {
    get(key: string): unknown;
    set(key: string, value: unknown): void;
    run(cb: (...args: unknown[]) => void): void;
  }
  export function getNamespace(name: string): Namespace | undefined;
  export function createNamespace(name: string): Namespace;
  export function destroyNamespace(name: string): void;
}
