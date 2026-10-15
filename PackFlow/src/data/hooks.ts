import { useEffect, useRef, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type {
  Action,
  Bootstrap,
  Mutation,
  OrderDetail,
  OrderSummary,
  Reports,
  ClientInsightsData,
  SalariesData,
  SeparateCostsData,
  OrderPage,
} from '../contracts';
import { repository } from './repository';

export const useBootstrap = () =>
  useQuery({
    queryKey: ['bootstrap'],
    queryFn: () => repository.query<Bootstrap>({ kind: 'bootstrap' }),
  });
export const useOrders = () =>
  useQuery({
    queryKey: ['orders'],
    queryFn: () => repository.query<OrderSummary[]>({ kind: 'list_orders' }),
  });
export const useOrder = (id: string | null) =>
  useQuery({
    queryKey: ['order', id],
    enabled: !!id,
    queryFn: () => repository.query<OrderDetail>({ kind: 'order', id: id! }),
  });
export const useReports = (from_date: string, to_date: string) =>
  useQuery({
    queryKey: ['reports', from_date, to_date],
    queryFn: () =>
      repository.query<Reports>({ kind: 'reports', from_date, to_date, include_portfolio: false }),
  });
export function useOrderPage(
  search: string,
  page: number,
  page_size: number,
  scope: 'all' | 'active',
) {
  const [term, setTerm] = useState(search);
  useEffect(() => {
    const timer = setTimeout(() => setTerm(search), 200);
    return () => clearTimeout(timer);
  }, [search]);
  const query = useQuery({
    queryKey: ['order-page', term, page, page_size, scope],
    queryFn: () =>
      repository.query<OrderPage>({ kind: 'order_page', search: term, page, page_size, scope }),
  });
  return { ...query, isSearching: term !== search };
}
export const useClientInsights = () =>
  useQuery({
    queryKey: ['client-insights'],
    queryFn: () => repository.query<ClientInsightsData>({ kind: 'client_insights' }),
  });
export const useSalaries = (month: string) =>
  useQuery({
    queryKey: ['salaries', month],
    enabled: /^\d{4}-\d{2}$/.test(month),
    queryFn: () => repository.query<SalariesData>({ kind: 'salaries', month }),
  });
export const useSeparateCosts = () =>
  useQuery({
    queryKey: ['separate-costs'],
    queryFn: () => repository.query<SeparateCostsData>({ kind: 'separate_costs' }),
  });
export function useWrite(actorId: string) {
  const client = useQueryClient();
  const uncertain = useRef<{ fingerprint: string; request: Mutation } | null>(null);
  return useMutation({
    mutationFn: async (action: Action) => {
      const fingerprint = JSON.stringify({ actorId, action });
      if (uncertain.current?.fingerprint !== fingerprint)
        uncertain.current = {
          fingerprint,
          request: { request_id: crypto.randomUUID(), actor_id: actorId, action },
        };
      const result = await repository.mutate(uncertain.current.request);
      uncertain.current = null;
      return result;
    },
    onSuccess: async () => {
      await client.invalidateQueries();
    },
  });
}
