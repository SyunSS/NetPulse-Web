import { defineStore } from 'pinia'
import { ref } from 'vue'
import { planApi, type PlanWithItems, type TaskPlanRun } from '@/api/plan'

export const usePlanStore = defineStore('plan', () => {
  const plans = ref<PlanWithItems[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref('')
  const currentPlan = ref<PlanWithItems | null>(null)
  const planRuns = ref<TaskPlanRun[]>([])
  const runsError = ref('')

  async function fetchPlans(page = 1, size = 20) {
    loading.value = true
    error.value = ''
    try {
      const res = await planApi.list(page, size)
      plans.value = res.data.plans
      total.value = res.data.total
    } catch (e) {
      error.value = e instanceof Error ? e.message : '加载计划失败'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchPlan(planId: string) {
    currentPlan.value = null
    const res = await planApi.get(planId)
    currentPlan.value = res.data
    return res.data
  }

  async function createPlan(data: any) {
    const res = await planApi.create(data)
    return res.data
  }

  async function updatePlan(planId: string, data: any) {
    const res = await planApi.update(planId, data)
    return res.data
  }

  async function deletePlan(planId: string) {
    await planApi.delete(planId)
  }

  async function runPlan(planId: string) {
    const res = await planApi.run(planId)
    return res.data
  }

  async function fetchPlanRuns(planId: string, params?: { start?: string; end?: string }) {
    runsError.value = ''
    try {
      const res = await planApi.runs(planId, params)
      planRuns.value = res.data
      return res.data
    } catch (e) {
      runsError.value = e instanceof Error ? e.message : '加载运行历史失败'
      throw e
    }
  }

  async function fetchAllPlans(size = 100) {
    loading.value = true
    error.value = ''
    try {
      const first = await planApi.list(1, size)
      const all = [...first.data.plans]
      const pages = Math.ceil(first.data.total / size)
      for (let page = 2; page <= pages; page += 1) {
        const res = await planApi.list(page, size)
        all.push(...res.data.plans)
      }
      plans.value = all
      total.value = first.data.total
    } catch (e) {
      error.value = e instanceof Error ? e.message : '加载计划失败'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteRun(planId: string, runId: string, force = false) {
    await planApi.deleteRun(planId, runId, force)
  }

  return {
    plans,
    total,
    loading,
    error,
    currentPlan,
    planRuns,
    runsError,
    fetchPlans,
    fetchAllPlans,
    fetchPlan,
    createPlan,
    updatePlan,
    deletePlan,
    runPlan,
    fetchPlanRuns,
    deleteRun,
  }
})
