<script lang="ts">
  import { Text } from '@threlte/extras';
  import { formatTime } from '../../../gameUtils';

  let { round, elapsedSeconds }: {
    round: number;
    elapsedSeconds: number;
  } = $props();

  let timeStr = $derived(formatTime(elapsedSeconds));
</script>

<Text
  text={`Round ${round} — ${timeStr}`}
  position={[0, 1.8, -1.5]}
  fontSize={0.08}
  color="#4a3a2a"
  anchorX="center"
  anchorY="middle"
  outlineWidth={0.004}
  outlineColor="#ffffff"
  fontWeight="bold"
/>
