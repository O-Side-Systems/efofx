import { createBrowserRouter, redirect } from 'react-router'
import Dashboard from './pages/Dashboard'
import Login from './pages/Login'
import { supabase } from './lib/supabase'

async function requireAuth() {
  const { data: { session } } = await supabase.auth.getSession()
  if (!session) throw redirect('/login')
  return null
}

export const router = createBrowserRouter([
  {
    path: '/login',
    element: <Login />,
  },
  {
    path: '/',
    loader: requireAuth,
    element: <Dashboard />,
  },
])
