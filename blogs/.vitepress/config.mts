import { defineConfig } from 'vitepress'
import { withSidebar } from 'vitepress-sidebar'
import { withMermaid } from "vitepress-plugin-mermaid"

// https://vitepress.dev/reference/site-config
export default withMermaid(withSidebar(
  defineConfig({
    title: "Embassy Preempt 博客",
    description: "基于 Rust 的嵌入式异步实时操作系统技术博客",
    themeConfig: {
      // https://vitepress.dev/reference/default-theme-config
      nav: [
        { text: '首页', link: '/' },
        { text: '文档', link: '/docs/'},
        { text: '技术报告', link: '/技术报告/' },
        { text: '周报', link: '/周报-Oveln/' },
        { text: '项目计划', link: '/项目计划/项目计划-20251111'}
      ],

      socialLinks: [
        { icon: 'github', link: 'https://github.com/Oveln/embassy_preempt' }
      ]
    }
  }),
  {
    // 侧边栏配置
    excludeByGlobPattern: ['node_modules/**', '.vitepress/**', 'public/**'],
    // sortMenusOrderByDescending: true,  // 从新到旧排序
    sortMenusByFrontmatterDate: true,
    useTitleFromFrontmatter: true      // 从 frontmatter 获取标题
  }
))