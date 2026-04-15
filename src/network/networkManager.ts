import { joinRoom } from 'trystero/nostr';
import type { Room } from 'trystero';
import type { HostMessage, GuestMessage } from './types';

// WebRTC ICE servers. Multiple STUN providers improve success rates behind most NATs,
// and the public openrelay TURN servers give a relay fallback for symmetric NATs where
// direct peer-to-peer fails. Openrelay is a free shared service with rate limits; for
// production traffic a dedicated TURN server is preferred.
const ICE_SERVERS: RTCIceServer[] = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun1.l.google.com:19302' },
  { urls: 'stun:stun2.l.google.com:19302' },
  { urls: 'stun:stun.cloudflare.com:3478' },
  {
    urls: [
      'turn:openrelay.metered.ca:80',
      'turn:openrelay.metered.ca:443',
      'turn:openrelay.metered.ca:443?transport=tcp',
    ],
    username: 'openrelayproject',
    credential: 'openrelayproject',
  },
];

const NOSTR_RELAYS = [
  'wss://relay.damus.io',
  'wss://nos.lol',
  'wss://relay.snort.social',
  'wss://nostr.mom',
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
    this.room = joinRoom({
      appId: 'colori-board-game',
      relayUrls: NOSTR_RELAYS,
      relayRedundancy: 3,
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
