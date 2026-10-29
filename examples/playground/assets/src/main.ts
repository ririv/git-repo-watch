import { greeting } from "./config";

export function welcome(name: string): string {
  return `${greeting}，${name}`;
}
