import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {ReservationRestriction} from "../types/wrapper";
import {dateToISOString} from "../utils/date";

export default function useRestrictionsQuery(since: Date | null = null): UseQueryResult<ReservationRestriction[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery(['reservations_restrictions', since ? dateToISOString(since) : null], async () => {
        let query = supabase.from('reservations_restrictions')
            .select()

        if (since != null)
            query = query.gte('date', dateToISOString(since))

        return query.then(result => result.data)
    })
}
