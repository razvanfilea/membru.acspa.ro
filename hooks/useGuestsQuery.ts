import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {GuestInvite, MemberTypes} from "../types/wrapper";

export default function useGuestsQuery(since: Date | null = null): UseQueryResult<GuestInvite[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery(['guests', since], async () => {
        let query = supabase.from('guest_invites')
            .select()

        if (since != null)
            query = query.gte('start_date', since)

        return query.order('start_date', {ascending: false})
            .order('start_hour', {ascending: true})
            .order('special', {ascending: false})
            .limit(50)
            .then(result => result.data)
    })
}
