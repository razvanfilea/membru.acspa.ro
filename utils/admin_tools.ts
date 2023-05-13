import {useEffect} from "react";
import {MemberTypes} from "../types/wrapper";
import {useProfile} from "../components/ProfileProvider";
import {useRouter} from "next/router";

export function useExitIfNotFounder() {
    const profileData = useProfile()
    const router = useRouter()

    useEffect(() => {
        console.log(profileData)
        if (!profileData.isLoading && (profileData.profile == null || profileData.profile.role !== MemberTypes.Fondator))
            router.back()
    }, [profileData, router])
}
