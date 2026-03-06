import { createBrowserRouter, redirect } from 'react-router'
import Dashboard from './pages/Dashboard'
import Login from './pages/Login'

function requireAuth() {
  const token = localStorage.getItem('access_token')
  if (!token) throw redirect('/login')
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
