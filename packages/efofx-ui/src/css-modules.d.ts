// CSS-module imports have no generated typings; consumers (Vite apps)
// resolve them at bundle time. This ambient declaration keeps
// `tsc --noEmit` (the CI type-check) in agreement with the bundler.
declare module "*.module.css" {
  const classes: { readonly [key: string]: string };
  export default classes;
}
