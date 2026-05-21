<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';

  interface BeforeInstallPromptEvent extends Event {
    prompt: () => Promise<void>;
    userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
  }

  let deferred = $state<BeforeInstallPromptEvent | null>(null);
  let isIOS = $state(false);
  let showIOSHint = $state(false);
  let visible = $state(false);

  function isStandalone() {
    return (
      window.matchMedia('(display-mode: standalone)').matches ||
      (navigator as unknown as { standalone?: boolean }).standalone === true
    );
  }

  onMount(() => {
    if (isStandalone()) return;

    const onBeforeInstall = (e: Event) => {
      e.preventDefault();
      deferred = e as BeforeInstallPromptEvent;
      visible = true;
    };
    const onInstalled = () => {
      visible = false;
      deferred = null;
    };

    window.addEventListener('beforeinstallprompt', onBeforeInstall);
    window.addEventListener('appinstalled', onInstalled);

    const ua = navigator.userAgent;
    if (/iphone|ipad|ipod/i.test(ua) && !/crios|fxios/i.test(ua)) {
      isIOS = true;
      visible = true;
    }

    return () => {
      window.removeEventListener('beforeinstallprompt', onBeforeInstall);
      window.removeEventListener('appinstalled', onInstalled);
    };
  });

  async function install() {
    if (isIOS) {
      showIOSHint = !showIOSHint;
      return;
    }
    if (!deferred) return;
    await deferred.prompt();
    await deferred.userChoice;
    deferred = null;
    visible = false;
  }
</script>

{#if visible}
  <div transition:fade={{ duration: 300 }} class="fixed bottom-6 left-6 z-[70] flex flex-col items-start gap-3 font-sans">
    {#if isIOS && showIOSHint}
      <div class="max-w-[260px] rounded-2xl border border-white/10 bg-black/90 backdrop-blur-xl p-4 text-[11px] leading-relaxed tracking-wide text-white/70 shadow-2xl">
        Tap the <span class="text-white font-bold">Share</span> icon, then
        <span class="text-white font-bold">Add to Home Screen</span> to install MDF.
      </div>
    {/if}
    <div class="flex items-center gap-2">
      <button
        onclick={install}
        class="flex items-center gap-2 rounded-full border border-white/20 bg-black/80 backdrop-blur-xl px-5 py-3 text-[10px] uppercase tracking-[0.25em] font-bold text-white/80 hover:text-white hover:border-white/40 transition-colors cursor-pointer shadow-2xl"
      >
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 3v12m0 0l-4-4m4 4l4-4M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2" />
        </svg>
        Install App
      </button>
      <button
        aria-label="Dismiss install prompt"
        onclick={() => (visible = false)}
        class="rounded-full border border-white/10 bg-black/60 backdrop-blur-xl w-9 h-9 flex items-center justify-center text-white/40 hover:text-white transition-colors cursor-pointer"
      >
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
  </div>
{/if}
