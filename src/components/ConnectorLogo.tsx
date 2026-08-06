import { Sparkle } from "@phosphor-icons/react";

export function ConnectorLogo({ clientId: _clientId }: { clientId: string }) {
  return <Sparkle className="client-logo__glyph" size={20} weight="duotone" />;
}
