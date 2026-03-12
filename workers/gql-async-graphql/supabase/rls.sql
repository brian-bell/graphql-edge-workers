-- Assumes public.flights already exists.
-- If the table already has rows, backfill user_id before SET NOT NULL.

alter table public.flights
    add column if not exists user_id uuid;

alter table public.flights
    alter column user_id set not null;

create index if not exists flights_user_id_idx
    on public.flights (user_id);

alter table public.flights enable row level security;

drop policy if exists "flights_select_own" on public.flights;
create policy "flights_select_own"
    on public.flights
    for select
    using (auth.uid() = user_id);

drop policy if exists "flights_insert_own" on public.flights;
create policy "flights_insert_own"
    on public.flights
    for insert
    with check (auth.uid() = user_id);

drop policy if exists "flights_update_own" on public.flights;
create policy "flights_update_own"
    on public.flights
    for update
    using (auth.uid() = user_id)
    with check (auth.uid() = user_id);

drop policy if exists "flights_delete_own" on public.flights;
create policy "flights_delete_own"
    on public.flights
    for delete
    using (auth.uid() = user_id);
