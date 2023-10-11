import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {GlobalVars} from "../types/wrapper";

export default function useGlobalVars(onSuccess: ((vars: GlobalVars) => void) | undefined = undefined): UseQueryResult<GlobalVars> {
    const supabase = useSupabaseClient<Database>()

    return useQuery({
            queryFn: async () => {
                return supabase.from('global_vars')
                    .select()
                    .single()
                    .then(result => result.data)
            },
            onSuccess
        }
    )
}
