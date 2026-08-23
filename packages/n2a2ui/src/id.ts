import type { ComponentId } from "@a2ui/web_core/v0_9";

export const ROOT_ID = "root" as const;

export function childId(
  parent: ComponentId,
  slot: string,
  index: number,
): ComponentId {
  return `${parent}::${slot}/${index}`;
}
