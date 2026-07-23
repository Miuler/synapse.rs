<script>
  import synapsersLogo2 from './assets/synapsers_logo_2.webp'
  import heroImg from './assets/hero.png'
  import Counter from './lib/Counter.svelte'
</script>

<section id="center">
  <div class="logo-card">
    <img src={synapsersLogo2} width="400" alt="" />
  </div>
  <div>
    <h1>Get started</h1>
    <p>Edit <code>src/App.svelte</code> and save to test <code>HMR</code></p>
  </div>
  <Counter />
</section>
