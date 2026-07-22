import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: () => import('@/views/Login.vue'),
      meta: { guest: true },
    },
    {
      path: '/',
      component: () => import('@/layouts/MainLayout.vue'),
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'Dashboard',
          component: () => import('@/views/Dashboard.vue'),
        },
        {
          path: 'create',
          name: 'CreateTask',
          component: () => import('@/views/CreateTask.vue'),
        },
        {
          path: 'plans',
          name: 'Plans',
          component: () => import('@/views/Plans.vue'),
        },
        {
          path: 'plans/new',
          name: 'PlanNew',
          component: () => import('@/views/PlanEdit.vue'),
        },
        {
          path: 'plans/:id/edit',
          name: 'PlanEdit',
          component: () => import('@/views/PlanEdit.vue'),
        },
        {
          path: 'plans/:id/runs',
          name: 'PlanRuns',
          component: () => import('@/views/PlanRuns.vue'),
        },
        {
          path: 'history',
          name: 'History',
          component: () => import('@/views/History.vue'),
        },
        {
          path: 'task/:id',
          name: 'TaskDetail',
          component: () => import('@/views/TaskDetail.vue'),
        },
        {
          path: 'settings',
          name: 'Settings',
          component: () => import('@/views/Settings.vue'),
        },
      ],
    },
  ],
})

// 路由守卫
router.beforeEach((to, _from, next) => {
  const authStore = useAuthStore()

  if (to.meta.requiresAuth && !authStore.isLoggedIn) {
    next({ path: '/login', replace: true })
  } else if (to.meta.guest && authStore.isLoggedIn) {
    next({ path: '/', replace: true })
  } else {
    next()
  }
})

export default router
