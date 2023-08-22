import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {ReservationRestriction} from "../types/wrapper";
import {dateToISOString} from "../utils/date";

export default function useRestrictionsQuery(date: Date | null = null): UseQueryResult<ReservationRestriction[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery(['reservations_restrictions', date], async () => {
        let query = supabase.from('reservations_restrictions')
            .select()

        if (date != null)
            query = query.gte('start_date', dateToISOString(date))

        return query.then(result => result.data)
    })
}
