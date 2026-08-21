export function isCurrentConnectorRefresh(
  generation: number,
  currentGeneration: number,
): boolean {
  return generation === currentGeneration;
}
