import React, {useCallback, useContext, useEffect, useState} from "react";
import {Profile} from "../../types/wrapper";
import {User, useSession, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";

export interface ProfileData {
    isLoading: boolean,
    profile: Profile | null
}

const defaultValue: ProfileData = {
    isLoading: true,
    profile: null,
}
const AuthContext = React.createContext(defaultValue)

export default function ProfileProvider({children}) {
    const supabase = useSupabaseClient<Database>()
    const session = useSession()
    const [profileData, setProfileData] = useState<ProfileData>({
        isLoading: true,
        profile: null
    })

    useEffect(() => {
        async function getProfile(user: User) {
            try {
                setProfileData({isLoading: true, profile: null})

                let {data, error} = await supabase
                    .from('profiles')
                    .select('*')
                    .eq('id', user.id)
                    .single()

                if (error) {
                    console.error(error)
                } else {
                    setProfileData({isLoading: false, profile: data})
                }

            } catch (error) {
                alert('Error loading user data!')
                console.error(error)
                setProfileData({isLoading: false, profile: null})
            }
        }

        if (session?.user) {
            getProfile(session!.user).then(() => console.log("Profile loaded successfully"))
        } else {
            console.log("Failed to get user session")
            setProfileData({isLoading: false, profile: null})
        }

        const {data: listener} = supabase.auth.onAuthStateChange(
            async (event) => {
                if (event == "PASSWORD_RECOVERY") {
                    let newPassword: string | null = null;
                    while (!newPassword || newPassword.length < 8)
                        newPassword = prompt("Noua ta parolă")

                    const {data, error} = await supabase.auth.updateUser({
                        password: newPassword,
                    })

                    if (data) alert("Parola ta a fost schimbată")
                    if (error) alert("A apărut o eroare la actualizarea parolei")
                }
            }
        )

        return () => {
            listener?.subscription?.unsubscribe()
        }
    }, [session, supabase]);

    // Will be passed down to Signup, Login and Dashboard components
    const authDataCallback = useCallback((): ProfileData => {
        return profileData
    }, [profileData])

    return (
        <AuthContext.Provider value={authDataCallback()}>
            {children}
        </AuthContext.Provider>
    )
}

export function useProfile() {
    return useContext(AuthContext)
}
