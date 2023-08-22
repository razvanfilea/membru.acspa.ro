import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {Profile} from "../types/wrapper";

export default function useProfilesQuery(): UseQueryResult<Profile[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery('profiles', async () => {
        return supabase.from('profiles')
            .select()
            .order('name', {ascending: true})
            .then(result => result.data)
    })
}
