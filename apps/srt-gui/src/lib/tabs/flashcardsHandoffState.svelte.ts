export class HandoffBox {
  current = $state<{ nonce: number; videoPath: string; subtitlePath: string } | null>(null);
}
