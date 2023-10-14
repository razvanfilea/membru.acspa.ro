import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {Guest} from "../types/wrapper";
import {dateToISOString} from "../utils/date";

export default function useGuestsQuery(since: Date | null = null): UseQueryResult<Guest[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery(['guests', since ? dateToISOString(since) : null], async () => {
        let query = supabase.from('guests')
            .select()

        if (since != null)
            query = query.gte('start_date', dateToISOString(since))

        return query.order('start_date', {ascending: false})
            .order('start_hour', {ascending: true})
            .order('created_at', {ascending: true})
            .order('special', {ascending: false})
            .limit(50)
            .then(result => result.data)
    })
}
