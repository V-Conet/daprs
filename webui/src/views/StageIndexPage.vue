<script setup lang="ts">
import { stageLinks } from '../lib/page'
</script>

<template>
  <div class="shell">
    <header class="topbar">
      <div>
        <p class="eyebrow">DAPRS / Flow</p>
        <h1>多阶段控制台</h1>
        <p class="lead">参考 docs/web 的操作节奏：登录、会话、节点、互联、队列，每一页独立入口。</p>
      </div>
    </header>

    <section class="card pane">
      <div class="pane-header">
        <div>
          <p class="eyebrow">Stages</p>
          <h3>页面入口</h3>
        </div>
      </div>
      <div class="stage-grid">
        <a v-for="item in stageLinks" :key="item.key" :href="item.href" class="stage-card">
          <strong>{{ item.label }}</strong>
          <span>{{ item.href }}</span>
        </a>
      </div>
    </section>
  </div>
</template>