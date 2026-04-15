import { joinRoom } from 'trystero/nostr';
import type { Room } from 'trystero';
import type { HostMessage, GuestMessage } from './types';

// WebRTC ICE servers — STUN only. One Google STUN + one Cloudflare STUN gives
// independent providers in case one is unreachable. No TURN fallback: the public
// openrelay credentials were deprecated in 2024 and a broken TURN entry is worse
// than none (WebRTC burns time on it before giving up). As a consequence, peers
// behind symmetric NAT or strict corporate firewalls can't establish a data
// channel. To fix that, add a TURN entry here pointing at your own relay —
// Cloudflare Realtime has a free tier, or self-host coturn.
const ICE_SERVERS: RTCIceServer[] = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun.cloudflare.com:3478' },
];

export class NetworkManager {
  private room: Room | null = null;
  private sendHostMsg: ((data: HostMessage, targetPeers?: string | string[]) => void) | null = null;
  private sendGuestMsg: ((data: GuestMessage, targetPeers?: string | string[]) => void) | null = null;
  peers: Set<string> = new Set();

  onPeerJoin: ((peerId: string) => void) | null = null;
  onPeerLeave: ((peerId: string) => void) | null = null;
  onHostMessage: ((msg: HostMessage, peerId: string) => void) | null = null;
  onGuestMessage: ((msg: GuestMessage, peerId: string) => void) | null = null;

  createRoom(): string {
    const code = generateRoomCode();
    this.join(code);
    return code;
  }

  join(code: string): void {
    // Let trystero pick nostr relays from its built-in list; hard-coding a small
    // subset led to outages when a chosen relay went down (e.g. snort.social).
    this.room = joinRoom({
      appId: 'colori-board-game',
      rtcConfig: { iceServers: ICE_SERVERS },
    }, code);

    this.room.onPeerJoin(peerId => {
      this.peers.add(peerId);
      this.onPeerJoin?.(peerId);
    });

    this.room.onPeerLeave(peerId => {
      this.peers.delete(peerId);
      this.onPeerLeave?.(peerId);
    });

    const [sendHost, getHost] = this.room.makeAction('host');
    const [sendGuest, getGuest] = this.room.makeAction('guest');

    this.sendHostMsg = sendHost as unknown as (data: HostMessage, targetPeers?: string | string[]) => void;
    this.sendGuestMsg = sendGuest as unknown as (data: GuestMessage, targetPeers?: string | string[]) => void;

    getHost((data, peerId) => this.onHostMessage?.(data as HostMessage, peerId));
    getGuest((data, peerId) => this.onGuestMessage?.(data as GuestMessage, peerId));
  }

  sendToHost(msg: GuestMessage): void {
    this.sendGuestMsg?.(msg);
  }

  sendToGuest(msg: HostMessage, peerId: string): void {
    this.sendHostMsg?.(msg, peerId);
  }

  sendToAllGuests(msg: HostMessage): void {
    this.sendHostMsg?.(msg);
  }

  sendToEachGuest(fn: (peerId: string) => HostMessage): void {
    for (const peerId of this.peers) {
      this.sendHostMsg?.(fn(peerId), peerId);
    }
  }

  leave(): void {
    this.room?.leave();
    this.room = null;
    this.peers.clear();
  }
}

function generateRoomCode(): string {
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  let code = '';
  for (let i = 0; i < 8; i++) {
    code += chars[bytes[i] % chars.length];
  }
  return code;
}
