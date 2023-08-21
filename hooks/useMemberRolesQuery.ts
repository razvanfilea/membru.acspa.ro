import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {useQuery, UseQueryResult} from "react-query";
import {MemberTypes} from "../types/wrapper";

export default function useMemberRolesQuery(): UseQueryResult<string[]> {
    const supabase = useSupabaseClient<Database>()

    return useQuery({
        queryKey: 'member_roles',
        queryFn: async () => {
            return supabase.from('member_roles')
                .select('role')
                .then(result => result.data?.map(it => it.role))
        },
        initialData: [MemberTypes.Membru]
    })
}
