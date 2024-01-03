import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {FreeDay} from "../types/wrapper";
import {dateToISOString} from "../utils/date";

export default function useFreeDaysQuery(since: Date | null = null): UseQueryResult<FreeDay[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery(['free_days', since ? dateToISOString(since) : null], async () => {
        let query = supabase.from('free_days')
            .select()

        if (since != null)
            query = query.gte('date', dateToISOString(since))

        return query.order('date', {ascending: false})
            .order('created_at', {ascending: true})
            .limit(50)
            .then(result => result.data)
    })
}
