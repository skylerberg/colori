import { joinRoom } from 'trystero/nostr';
import type { Room } from 'trystero';
import type { HostMessage, GuestMessage } from './types';

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
    this.room = joinRoom({ appId: 'colori-board-game' }, code);

    this.room.onPeerJoin(peerId => {
      this.peers.add(peerId);
      this.onPeerJoin?.(peerId);
    });

    this.room.onPeerLeave(peerId => {
      this.peers.delete(peerId);
      this.onPeerLeave?.(peerId);
    });

    const [sendHost, getHost] = this.room.makeAction<HostMessage>('host');
    const [sendGuest, getGuest] = this.room.makeAction<GuestMessage>('guest');

    this.sendHostMsg = sendHost as (data: HostMessage, targetPeers?: string | string[]) => void;
    this.sendGuestMsg = sendGuest as (data: GuestMessage, targetPeers?: string | string[]) => void;

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
  let code = '';
  for (let i = 0; i < 6; i++) {
    code += chars[Math.floor(Math.random() * chars.length)];
  }
  return code;
}
