/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

declare module 'mammoth' {
  export interface ConvertResult {
    value: string
    messages: any[]
  }
  export function convertToHtml(options: { arrayBuffer: ArrayBuffer }): Promise<ConvertResult>
}
