<script lang="ts">
  import { BOARD_ART_WIDTH, PLAYER_BOARD_HEIGHT, PORTRAIT_FACES, PORTRAIT_SIZE } from '../data/boardLayout';
  import { playerBoardArtUrl, seatCharacter } from '../data/cardArt';

  // The seat's character, cut out of its player board.
  let { seat, size = 32 }: { seat: number; size?: number } = $props();

  let face = $derived(PORTRAIT_FACES[seatCharacter(seat) - 1]);
  let scale = $derived(size / PORTRAIT_SIZE);
</script>

<span
  class="portrait"
  style:width="{size}px"
  style:height="{size}px"
  style:background-image="url('{playerBoardArtUrl(seat)}')"
  style:background-size="{BOARD_ART_WIDTH * scale}px {PLAYER_BOARD_HEIGHT * scale}px"
  style:background-position="{-(face.x - PORTRAIT_SIZE / 2) * scale}px {-(face.y - PORTRAIT_SIZE / 2) * scale}px"
></span>

<style>
  .portrait {
    display: inline-block;
    flex-shrink: 0;
    border-radius: 50%;
    background-repeat: no-repeat;
    border: 2px solid rgba(201, 168, 76, 0.7);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
  }
</style>
