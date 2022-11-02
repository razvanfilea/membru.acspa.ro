import MyAppShell from "./MyAppShell";
import {SupabaseClient} from "@supabase/supabase-js";

export default MyAppShell;

export async function changePasswordAsync(supabase: SupabaseClient, userName: string | null, password: string): Promise<boolean> {
    console.log("Updating password")
    const {error, data} = await supabase.auth.updateUser({password})
    const user = data.user

    if (user != null) {
        if (userName != null) {
            console.log("Updating profile")

            const profile = {name: userName}

            await supabase.from('profiles').insert([profile])
        }
        return true
    }
    console.log("Failed to change password", error)

    return false
}
